use super::*;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tiktools_plugin_loader::{
    PluginInstance, PluginLoaderError, PluginRoot, PluginRuntime, PluginSource, RuntimeRegistry,
};
use tokio::sync::Semaphore;

type Handler = Arc<dyn Fn(&[u8]) -> Result<Vec<u8>, PluginLoaderError> + Send + Sync>;

struct FakeRuntime {
    handler: Handler,
}

impl PluginRuntime for FakeRuntime {
    fn kind(&self) -> tiktools_plugin_api::manifest::PluginRuntimeKind {
        tiktools_plugin_api::manifest::PluginRuntimeKind::Process
    }

    fn load(
        &self,
        manifest: &tiktools_plugin_api::manifest::PluginManifest,
        _directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        Ok(Box::new(FakeInstance {
            id: manifest.id.clone(),
            handler: Arc::clone(&self.handler),
        }))
    }
}

struct FakeInstance {
    id: String,
    handler: Handler,
}

impl PluginInstance for FakeInstance {
    fn id(&self) -> &str {
        &self.id
    }

    fn handle_message(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
        (self.handler)(request)
    }

    fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
        Ok(())
    }
}

static HARNESS_COUNTER: AtomicU64 = AtomicU64::new(0);

struct Harness {
    plugins: Arc<PluginManager>,
    capabilities: CapabilityBroker,
    health: Mutex<BTreeMap<ProcessorKey, PluginHealth>>,
    metrics: Mutex<BTreeMap<ProcessorKey, ProcessorMetrics>>,
    slots: Arc<Semaphore>,
}

fn scripted(
    handler: impl Fn(&[u8]) -> Result<Vec<u8>, PluginLoaderError> + Send + Sync + 'static,
) -> Handler {
    Arc::new(handler)
}

fn enrich_response(annotations: Value, views: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "annotations": annotations,
        "views": views,
        "logs": ["analyzed"],
    }))
    .unwrap()
}

fn chat_event() -> Value {
    json!({
        "id": "evt-1",
        "type": "tiktok.chat",
        "timestamp": 1,
        "connectionId": "connection-1",
        "creator": {"uniqueId": "creator", "roomId": "1"},
        "user": {"uniqueId": "alice", "nickname": "Alice", "secUid": "", "userId": "7"},
        "data": {"comment": "Hello there", "method": "m", "msgId": "1", "isHistory": false}
    })
}

fn processor_manifest(id: &str, capabilities: &[&str], processors: Value) -> Value {
    json!({
        "schemaVersion": 2,
        "id": id,
        "name": id,
        "version": "0.1.0",
        "runtime": "process",
        "entry": "entry.bin",
        "capabilities": capabilities,
        "processorTypes": processors,
    })
}

fn make_harness(manifests: &[(&str, Value)], handler: Handler) -> Harness {
    let tag = HARNESS_COUNTER.fetch_add(1, Ordering::AcqRel);
    let root = std::env::temp_dir().join(format!(
        "tiktools-processor-test-{}-{tag}",
        std::process::id()
    ));
    for (id, manifest) in manifests {
        let directory = root.join(id);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("plugin.json"),
            serde_json::to_vec_pretty(manifest).unwrap(),
        )
        .unwrap();
        std::fs::write(directory.join("entry.bin"), b"fake").unwrap();
    }
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(FakeRuntime { handler }) as Arc<dyn PluginRuntime>);
    let plugins = Arc::new(PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        runtimes,
    ));
    plugins.scan().unwrap();
    Harness {
        plugins,
        capabilities: CapabilityBroker::new(root.join("data")),
        health: Mutex::new(BTreeMap::new()),
        metrics: Mutex::new(BTreeMap::new()),
        slots: Arc::new(Semaphore::new(MAX_TOTAL_PROCESSOR_SLOTS)),
    }
}

fn drop_harness(harness: Harness) {
    harness.plugins.stop_all();
    let _ = harness;
}

fn invoker_for(harness: &Harness) -> crate::plugin_invoker::PluginInvoker {
    crate::plugin_invoker::PluginInvoker::new(Arc::clone(&harness.plugins))
}

async fn enrich(harness: &Harness, event: Value) -> Value {
    let event_type = event
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let eligible = collect_eligible_processors(
        &harness.plugins,
        &harness.capabilities,
        |_| true,
        &event_type,
    );
    let invoker = invoker_for(harness);
    let outcomes = execute_processors(
        &invoker,
        &harness.health,
        &harness.metrics,
        &harness.slots,
        |_| Value::Null,
        eligible,
        &event,
    )
    .await;
    merge_processor_outcomes(event, &outcomes)
}

fn raw_fields(event: &Value) -> Value {
    let mut event = event.clone();
    if let Some(object) = event.as_object_mut() {
        object.remove("intel");
    }
    event
}

#[tokio::test]
async fn eligible_processor_enriches_matching_event() {
    let manifest = processor_manifest(
        "textintel",
        &["events.enrich"],
        json!([{
            "id": "textintel.analyze",
            "title": {"default": "Text Intelligence"},
            "eventTypes": ["tiktok.chat"],
        }]),
    );
    let harness = make_harness(
        &[("textintel", manifest)],
        scripted(|request| {
            let call: tiktools_plugin_sdk::PluginCall = serde_json::from_slice(request).unwrap();
            let request = call.into_enrich().expect("must be an enrich call");
            assert_eq!(request.processor_id, "textintel.analyze");
            assert_eq!(request.event["data"]["comment"], "Hello there");
            Ok(enrich_response(
                json!({"comment": {"normalized": "hello there"}}),
                json!({"tts": {"text": "hello there", "source": "normalized"}}),
            ))
        }),
    );
    let event = chat_event();
    let enriched = enrich(&harness, event.clone()).await;
    assert_eq!(raw_fields(&enriched), raw_fields(&event));
    assert_eq!(enriched["intel"]["comment"]["normalized"], "hello there");
    assert_eq!(
        enriched["intel"]["providers"]["textintel"]["comment"]["normalized"],
        "hello there"
    );
    assert_eq!(
        enriched["intel"]["providers"]["textintel"]["views"]["tts"]["text"],
        "hello there"
    );
    assert!(enriched["intel"].get("processing").is_none());
    drop_harness(harness);
}

#[tokio::test]
async fn enrich_requests_carry_plugin_settings() {
    let manifest = processor_manifest(
        "configured",
        &["events.enrich"],
        json!([{"id": "configured.analyze", "title": {"default": "Configured"}}]),
    );
    let harness = make_harness(
        &[("configured", manifest)],
        scripted(|request| {
            let call: tiktools_plugin_sdk::PluginCall = serde_json::from_slice(request).unwrap();
            let request = call.into_enrich().unwrap();
            assert_eq!(request.settings["analyzeComments"], false);
            Ok(enrich_response(json!({}), json!({})))
        }),
    );
    let event = chat_event();
    let eligible = collect_eligible_processors(
        &harness.plugins,
        &harness.capabilities,
        |_| true,
        "tiktok.chat",
    );
    assert_eq!(eligible.len(), 1);
    let invoker = invoker_for(&harness);
    let outcomes = execute_processors(
        &invoker,
        &harness.health,
        &harness.metrics,
        &harness.slots,
        |_| json!({"analyzeComments": false}),
        eligible,
        &event,
    )
    .await;
    assert!(outcomes[0].3.is_ok());
    drop_harness(harness);
}

#[tokio::test]
async fn processor_not_matching_event_type_is_skipped() {
    let manifest = processor_manifest(
        "gifts",
        &["events.enrich"],
        json!([{
            "id": "gifts.describe",
            "title": {"default": "Gifts"},
            "eventTypes": ["tiktok.gift"],
        }]),
    );
    let harness = make_harness(
        &[("gifts", manifest)],
        scripted(|_| panic!("must not be called")),
    );
    let event = chat_event();
    let enriched = enrich(&harness, event.clone()).await;
    assert_eq!(enriched, event);
    drop_harness(harness);
}

#[tokio::test]
async fn missing_capability_and_disabled_plugins_are_skipped() {
    let without_capability = processor_manifest(
        "nocap",
        &[],
        json!([{"id": "nocap.analyze", "title": {"default": "No cap"}}]),
    );
    let harness = make_harness(
        &[("nocap", without_capability)],
        scripted(|_| panic!("must not be called")),
    );
    let event = chat_event();
    assert!(collect_eligible_processors(
        &harness.plugins,
        &harness.capabilities,
        |_| true,
        "tiktok.chat"
    )
    .is_empty());
    assert_eq!(enrich(&harness, event.clone()).await, event);

    let disabled = processor_manifest(
        "off",
        &["events.enrich"],
        json!([{"id": "off.analyze", "title": {"default": "Off"}}]),
    );
    let harness = make_harness(
        &[("off", disabled)],
        scripted(|_| panic!("must not be called")),
    );
    assert!(collect_eligible_processors(
        &harness.plugins,
        &harness.capabilities,
        |_| false,
        "tiktok.chat"
    )
    .is_empty());
    drop_harness(harness);
}

#[tokio::test]
async fn processor_timeout_passes_raw_event_through() {
    let manifest = processor_manifest(
        "slow",
        &["events.enrich"],
        json!([{
            "id": "slow.analyze",
            "title": {"default": "Slow"},
            "timeoutMs": 5,
        }]),
    );
    let harness = make_harness(
        &[("slow", manifest)],
        scripted(|_| {
            std::thread::sleep(Duration::from_millis(300));
            Ok(enrich_response(json!({}), json!({})))
        }),
    );
    let event = chat_event();
    let enriched = enrich(&harness, event.clone()).await;
    assert_eq!(raw_fields(&enriched), raw_fields(&event));
    assert_eq!(enriched["intel"]["processing"]["status"], "degraded");
    assert!(enriched["intel"].get("providers").is_none());
    let metrics = harness.metrics.lock().unwrap();
    let metric = &metrics[&ProcessorKey::new("slow", "slow.analyze")];
    assert_eq!(metric.timeouts, 1);
    assert_eq!(metric.failures, 1);
    drop(metrics);
    drop_harness(harness);
}

#[tokio::test]
async fn processor_errors_pass_raw_event_through() {
    for (tag, handler) in [
        (
            "crash",
            scripted(|_| Err(PluginLoaderError::Runtime("boom".to_owned()))),
        ),
        ("invalid-json", scripted(|_| Ok(b"not json".to_vec()))),
        (
            "side-effect",
            scripted(|_| {
                Ok(serde_json::to_vec(&json!({
                    "annotations": {},
                    "emit": [{"type": "x", "data": {}}],
                }))
                .unwrap())
            }),
        ),
    ] {
        let manifest = processor_manifest(
            tag,
            &["events.enrich"],
            json!([{"id": format!("{tag}.analyze"), "title": {"default": tag}}]),
        );
        // Rebind per-case plugin id for the manifest directory layout.
        let id = tag.to_owned();
        let manifest = {
            let mut manifest = manifest;
            manifest["id"] = Value::String(id.clone());
            manifest["name"] = Value::String(id.clone());
            manifest
        };
        let harness = make_harness(&[(tag, manifest)], handler);
        let event = chat_event();
        let enriched = enrich(&harness, event.clone()).await;
        assert_eq!(raw_fields(&enriched), raw_fields(&event), "{tag}");
        assert_eq!(
            enriched["intel"]["processing"]["status"], "degraded",
            "{tag}"
        );
        drop_harness(harness);
    }
}

#[tokio::test]
async fn two_processors_merge_deterministically() {
    let first = processor_manifest(
        "aaa",
        &["events.enrich"],
        json!([{"id": "aaa.analyze", "title": {"default": "A"}}]),
    );
    let second = processor_manifest(
        "zzz",
        &["events.enrich"],
        json!([{"id": "zzz.analyze", "title": {"default": "Z"}}]),
    );
    let harness = make_harness(
        &[("zzz", second), ("aaa", first)],
        scripted(|request| {
            let call: tiktools_plugin_sdk::PluginCall = serde_json::from_slice(request).unwrap();
            let request = call.into_enrich().unwrap();
            let tag = if request.processor_id.starts_with("aaa") {
                "aaa"
            } else {
                "zzz"
            };
            let mut comment = serde_json::Map::new();
            comment.insert("normalized".to_owned(), Value::String(tag.to_owned()));
            comment.insert(format!("only-{tag}"), Value::Bool(true));
            Ok(enrich_response(
                json!({"comment": Value::Object(comment)}),
                json!({}),
            ))
        }),
    );
    let event = chat_event();
    let enriched = enrich(&harness, event.clone()).await;
    // Both providers stay independent under their namespaces; the stable
    // key resolves by first-writer-wins in selection order, in canonical
    // contract form (unknown keys never promote).
    assert_eq!(
        enriched["intel"]["providers"]["aaa"]["comment"]["normalized"],
        "aaa"
    );
    assert_eq!(
        enriched["intel"]["providers"]["zzz"]["comment"]["normalized"],
        "zzz"
    );
    assert_eq!(enriched["intel"]["comment"]["normalized"], "aaa");
    assert!(enriched["intel"]["comment"].get("only-aaa").is_none());
    assert!(enriched["intel"]["comment"].get("only-zzz").is_none());
    drop_harness(harness);
}

#[tokio::test]
async fn higher_priority_processor_owns_the_stable_key() {
    let low = processor_manifest(
        "aaa",
        &["events.enrich"],
        json!([{"id": "aaa.analyze", "title": {"default": "A"}, "priority": 0}]),
    );
    let high = processor_manifest(
        "zzz",
        &["events.enrich"],
        json!([{"id": "zzz.analyze", "title": {"default": "Z"}, "priority": 10}]),
    );
    let harness = make_harness(
        &[("aaa", low), ("zzz", high)],
        scripted(|request| {
            let call: tiktools_plugin_sdk::PluginCall = serde_json::from_slice(request).unwrap();
            let request = call.into_enrich().unwrap();
            let tag = if request.processor_id.starts_with("aaa") {
                "aaa"
            } else {
                "zzz"
            };
            Ok(enrich_response(
                json!({"comment": {"normalized": tag}}),
                json!({}),
            ))
        }),
    );
    let eligible = collect_eligible_processors(
        &harness.plugins,
        &harness.capabilities,
        |_| true,
        "tiktok.chat",
    );
    assert_eq!(eligible[0].plugin_id, "zzz");
    assert_eq!(eligible[1].plugin_id, "aaa");
    let enriched = enrich(&harness, chat_event()).await;
    assert_eq!(enriched["intel"]["comment"]["normalized"], "zzz");
    drop_harness(harness);
}

#[tokio::test]
async fn descriptor_inputs_resolve_to_roles_in_the_request() {
    let manifest = processor_manifest(
        "inputs",
        &["events.enrich"],
        json!([{
            "id": "inputs.analyze",
            "title": {"default": "Inputs"},
            "inputs": [
                {"path": "event.data.comment", "role": "message"},
                {"path": "event.user.nickname", "role": "display-name"},
                {"path": "event.data.missing", "role": "absent"},
            ],
        }]),
    );
    let harness = make_harness(
        &[("inputs", manifest)],
        scripted(|request| {
            let call: tiktools_plugin_sdk::PluginCall = serde_json::from_slice(request).unwrap();
            let request = call.into_enrich().unwrap();
            let inputs = request.inputs.expect("host resolves inputs");
            assert_eq!(inputs["message"], "Hello there");
            assert_eq!(inputs["display-name"], "Alice");
            assert_eq!(inputs["absent"], Value::Null);
            Ok(enrich_response(json!({}), json!({})))
        }),
    );
    let enriched = enrich(&harness, chat_event()).await;
    assert!(enriched["intel"].get("processing").is_none());
    drop_harness(harness);
}

#[test]
fn input_paths_use_the_shared_event_path_language() {
    let descriptor = tiktools_plugin_api::manifest::PluginProcessorDescriptor {
        id: "demo.analyze".to_owned(),
        title: serde_json::json!({"default": "Demo"}),
        description: None,
        event_types: vec![],
        inputs: vec![
            tiktools_plugin_api::manifest::ProcessorInputDescriptor {
                path: "event.data.comment".to_owned(),
                role: "message".to_owned(),
            },
            tiktools_plugin_api::manifest::ProcessorInputDescriptor {
                path: "data.comment".to_owned(),
                role: "relative".to_owned(),
            },
            tiktools_plugin_api::manifest::ProcessorInputDescriptor {
                path: "event.data.missing".to_owned(),
                role: "absent".to_owned(),
            },
        ],
        stage: Default::default(),
        failure_mode: Default::default(),
        timeout_ms: None,
        priority: None,
    };
    let resolved = resolve_processor_inputs(&descriptor, &chat_event());
    assert_eq!(resolved["message"], "Hello there");
    assert_eq!(resolved["relative"], "Hello there");
    assert_eq!(resolved["absent"], Value::Null);
}

#[tokio::test]
async fn oversized_enrichment_result_is_rejected() {
    let manifest = processor_manifest(
        "big",
        &["events.enrich"],
        json!([{"id": "big.analyze", "title": {"default": "Big"}}]),
    );
    let harness = make_harness(
        &[("big", manifest)],
        scripted(|_| {
            Ok(enrich_response(
                json!({"comment": {"blob": "x".repeat(40 * 1024)}}),
                json!({}),
            ))
        }),
    );
    let event = chat_event();
    let enriched = enrich(&harness, event.clone()).await;
    assert_eq!(raw_fields(&enriched), raw_fields(&event));
    assert_eq!(enriched["intel"]["processing"]["status"], "degraded");
    assert!(enriched["intel"].get("providers").is_none());
    drop_harness(harness);
}

#[tokio::test]
async fn circuit_breaker_opens_and_recovers() {
    assert_eq!(plugin_backoff_seconds(1), 1);
    assert_eq!(plugin_backoff_seconds(2), 2);
    assert_eq!(plugin_backoff_seconds(4), 10);
    assert_eq!(plugin_backoff_seconds(9), 30);
    let manifest = processor_manifest(
        "flaky",
        &["events.enrich"],
        json!([{"id": "flaky.analyze", "title": {"default": "Flaky"}}]),
    );
    let calls = Arc::new(AtomicU64::new(0));
    let calls_for_handler = Arc::clone(&calls);
    let harness = make_harness(
        &[("flaky", manifest)],
        scripted(move |_| {
            calls_for_handler.fetch_add(1, Ordering::AcqRel);
            Err(PluginLoaderError::Runtime("boom".to_owned()))
        }),
    );
    let event = chat_event();
    let key = ProcessorKey::new("flaky", "flaky.analyze");
    let enriched = enrich(&harness, event.clone()).await;
    assert_eq!(enriched["intel"]["processing"]["status"], "degraded");
    assert_eq!(calls.load(Ordering::Acquire), 1);
    assert!(!processor_retry_allowed(&harness.health, &key));

    // While the circuit is open the plugin is not called again.
    let enriched = enrich(&harness, event.clone()).await;
    assert_eq!(enriched["intel"]["processing"]["status"], "degraded");
    assert_eq!(calls.load(Ordering::Acquire), 1);
    let metrics = harness.metrics.lock().unwrap();
    assert_eq!(metrics[&key].skipped_circuit_open, 1);
    drop(metrics);

    // A successful retry closes the circuit.
    record_processor_success(&harness.health, &key);
    assert!(processor_retry_allowed(&harness.health, &key));
    drop_harness(harness);
}

#[tokio::test]
async fn single_processor_preview_reports_typed_outcomes() {
    let manifest = processor_manifest(
        "demo",
        &["events.enrich"],
        json!([{"id": "demo.analyze", "title": {"default": "Demo"}}]),
    );
    let harness = make_harness(
        &[("demo", manifest)],
        scripted(|_| Ok(enrich_response(json!({"comment": {"ok": true}}), json!({})))),
    );
    let invoker = invoker_for(&harness);
    let outcome = run_single_processor(
        &invoker,
        &harness.capabilities,
        &harness.health,
        &harness.metrics,
        |_| true,
        |_| Value::Null,
        "demo",
        "demo.analyze",
        chat_event(),
    )
    .await;
    assert!(outcome.ok);
    assert!(outcome.error.is_none());
    assert_eq!(outcome.result["annotations"]["comment"]["ok"], true);

    let missing = run_single_processor(
        &invoker,
        &harness.capabilities,
        &harness.health,
        &harness.metrics,
        |_| true,
        |_| Value::Null,
        "missing",
        "missing.analyze",
        chat_event(),
    )
    .await;
    assert!(!missing.ok);
    assert!(missing.error.unwrap_or_default().contains("not installed"));

    let unknown = run_single_processor(
        &invoker,
        &harness.capabilities,
        &harness.health,
        &harness.metrics,
        |_| true,
        |_| Value::Null,
        "demo",
        "demo.unknown",
        chat_event(),
    )
    .await;
    assert!(!unknown.ok);
    assert!(unknown.error.unwrap_or_default().contains("not declared"));

    let shaped = run_single_processor(
        &invoker,
        &harness.capabilities,
        &harness.health,
        &harness.metrics,
        |_| true,
        |_| Value::Null,
        "demo",
        "demo.analyze",
        Value::String("nope".to_owned()),
    )
    .await;
    assert!(!shaped.ok);
    drop_harness(harness);
}

#[tokio::test]
async fn status_snapshot_reports_ready_and_degraded() {
    let manifest = processor_manifest(
        "demo",
        &["events.enrich"],
        json!([
            {"id": "demo.analyze", "title": {"default": "Demo"}, "eventTypes": ["tiktok.chat"]},
            {"id": "BAD ID", "title": {"default": "Bad"}},
        ]),
    );
    let failing = processor_manifest(
        "broken",
        &["events.enrich"],
        json!([
            {"id": "broken.analyze", "title": {"default": "Broken"}},
        ]),
    );
    let harness = make_harness(
        &[("demo", manifest), ("broken", failing)],
        scripted(|_| Err(PluginLoaderError::Runtime("boom".to_owned()))),
    );
    // Invalid descriptors never appear in the catalog.
    let snapshot = processor_status_entries(
        &harness.plugins,
        &harness.capabilities,
        |_| true,
        &harness.health,
        &harness.metrics,
    );
    assert_eq!(snapshot.as_array().unwrap().len(), 2);
    assert_eq!(
        snapshot[0]["processorId"],
        "broken/broken.analyze".split('/').nth(1).unwrap()
    );
    assert_eq!(snapshot[0]["status"], "ready");

    // After a failure the entry degrades and then opens its circuit.
    let _ = enrich(&harness, chat_event()).await;
    let snapshot = processor_status_entries(
        &harness.plugins,
        &harness.capabilities,
        |_| true,
        &harness.health,
        &harness.metrics,
    );
    assert_eq!(snapshot[0]["status"], "circuit-open");
    assert_eq!(snapshot[0]["metrics"]["failures"], 1);

    // Disabled plugins report as disabled.
    let snapshot = processor_status_entries(
        &harness.plugins,
        &harness.capabilities,
        |_| false,
        &harness.health,
        &harness.metrics,
    );
    assert!(snapshot
        .as_array()
        .unwrap()
        .iter()
        .all(|entry| entry["status"] == "disabled"));
    drop_harness(harness);
}

#[tokio::test]
async fn processor_fan_out_stays_within_its_concurrency_limit() {
    let manifests: Vec<(String, Value)> = (0..8)
        .map(|index| {
            let id = format!("burst{index}");
            let manifest = processor_manifest(
                &id,
                &["events.enrich"],
                json!([{"id": format!("{id}.analyze"), "title": {"default": id}}]),
            );
            (id, manifest)
        })
        .collect();
    let in_flight = Arc::new(AtomicU64::new(0));
    let max_in_flight = Arc::new(AtomicU64::new(0));
    let in_flight_for_handler = Arc::clone(&in_flight);
    let max_for_handler = Arc::clone(&max_in_flight);
    let manifest_refs: Vec<(&str, Value)> = manifests
        .iter()
        .map(|(id, manifest)| (id.as_str(), manifest.clone()))
        .collect();
    let harness = make_harness(
        &manifest_refs,
        scripted(move |_| {
            let current = in_flight_for_handler.fetch_add(1, Ordering::AcqRel) + 1;
            max_for_handler.fetch_max(current, Ordering::AcqRel);
            std::thread::sleep(Duration::from_millis(20));
            in_flight_for_handler.fetch_sub(1, Ordering::AcqRel);
            Ok(enrich_response(json!({"comment": {"ok": true}}), json!({})))
        }),
    );
    let event = chat_event();
    let enriched = enrich(&harness, event.clone()).await;
    assert_eq!(raw_fields(&enriched), raw_fields(&event));
    assert!(enriched["intel"]["comment"].is_object());
    let max = max_in_flight.load(Ordering::Acquire);
    assert!(
        max <= MAX_CONCURRENT_PROCESSORS as u64,
        "in-flight peak {max} exceeds the limit"
    );
    assert!(max > 1, "burst did not overlap; the limit was not stressed");
    drop_harness(harness);
}

#[tokio::test]
async fn chat_burst_flows_raw_while_a_processor_fails() {
    let manifest = processor_manifest(
        "down",
        &["events.enrich"],
        json!([{"id": "down.analyze", "title": {"default": "Down"}}]),
    );
    let calls = Arc::new(AtomicU64::new(0));
    let calls_for_handler = Arc::clone(&calls);
    let harness = make_harness(
        &[("down", manifest)],
        scripted(move |_| {
            calls_for_handler.fetch_add(1, Ordering::AcqRel);
            Err(PluginLoaderError::Runtime("boom".to_owned()))
        }),
    );
    for index in 0..50 {
        let mut event = chat_event();
        event["id"] = Value::String(format!("evt-{index}"));
        let enriched = enrich(&harness, event.clone()).await;
        assert_eq!(raw_fields(&enriched), raw_fields(&event), "event {index}");
        assert_eq!(enriched["intel"]["processing"]["status"], "degraded");
    }
    // The first failure opens the circuit; the burst does not retry it.
    assert_eq!(calls.load(Ordering::Acquire), 1);
    drop_harness(harness);
}

#[test]
fn merge_validates_stable_keys_and_reserves_views() {
    let event = chat_event();
    let mut result = tiktools_plugin_sdk::EventEnrichmentResult::default();
    result.annotations.insert(
        "comment".to_owned(),
        Value::String("not-an-object".to_owned()),
    );
    result
        .annotations
        .insert("views".to_owned(), json!({"smuggled": true}));
    result.views.insert(
        "comment".to_owned(),
        tiktools_plugin_sdk::TextView::new("hello", "raw"),
    );
    let enriched = merge_processor_outcomes(
        event,
        &[(
            0,
            "demo".to_owned(),
            "demo.analyze".to_owned(),
            Ok(TimedEnrichment {
                result,
                duration_ms: 3,
            }),
        )],
    );
    // Invalid stable contributions stay provider-namespaced only and mark
    // the event degraded.
    assert_eq!(
        enriched["intel"]["providers"]["demo"]["comment"],
        "not-an-object"
    );
    assert_eq!(enriched["intel"]["processing"]["status"], "degraded");
    // The reserved views key is never overwritten by annotations.
    assert_eq!(
        enriched["intel"]["providers"]["demo"]["views"]["comment"]["text"],
        "hello"
    );
    // Views still project: the failed annotation does not block the TTS
    // projection from the same result.
    assert_eq!(enriched["intel"]["comment"]["tts"]["text"], "hello");
    assert!(enriched["intel"]["comment"].get("normalized").is_none());
}

#[test]
fn annotation_tts_never_promotes_views_are_canonical() {
    let event = chat_event();
    let mut result = tiktools_plugin_sdk::EventEnrichmentResult::default();
    result.annotations.insert(
        "comment".to_owned(),
        json!({"normalized": "hi", "tts": {"text": "smuggled", "source": "annotations"}}),
    );
    let enriched = merge_processor_outcomes(
        event,
        &[(
            0,
            "demo".to_owned(),
            "demo.analyze".to_owned(),
            Ok(TimedEnrichment {
                result,
                duration_ms: 1,
            }),
        )],
    );
    assert_eq!(enriched["intel"]["comment"]["normalized"], "hi");
    assert!(enriched["intel"]["comment"].get("tts").is_none());
    assert!(enriched["intel"].get("processing").is_none());
}

#[test]
fn contribution_index_caches_selection_order_off_the_hot_path() {
    let chat_only = processor_manifest(
        "chatty",
        &["events.enrich"],
        json!([{
            "id": "chatty.analyze",
            "title": {"default": "Chatty"},
            "eventTypes": ["tiktok.chat"],
            "priority": 1,
        }]),
    );
    let catch_all = processor_manifest(
        "aaa",
        &["events.enrich"],
        json!([{
            "id": "aaa.analyze",
            "title": {"default": "Aaa"},
            "priority": 9,
        }]),
    );
    let harness = make_harness(
        &[("chatty", chat_only), ("aaa", catch_all)],
        scripted(|_| panic!("index build must not call plugins")),
    );
    let index = ContributionIndex::build(&harness.plugins, &harness.capabilities, |_| true);
    assert_eq!(index.len(), 2);
    assert!(!index.is_empty());

    // Selection order is priority-first regardless of subscription: the
    // catch-all outranks the chat-only processor on chat events too.
    let chat = index.eligible_for("tiktok.chat");
    assert_eq!(chat.len(), 2);
    assert_eq!(chat[0].processor_id, "aaa.analyze");
    assert_eq!(chat[1].processor_id, "chatty.analyze");

    // Other event types see only the catch-all subscription.
    let gift = index.eligible_for("tiktok.gift");
    assert_eq!(gift.len(), 1);
    assert_eq!(gift[0].processor_id, "aaa.analyze");

    // Rebuilding with a disabled plugin drops it without touching manifests.
    let disabled =
        ContributionIndex::build(&harness.plugins, &harness.capabilities, |id| id != "aaa");
    assert_eq!(disabled.len(), 1);
    assert_eq!(disabled.eligible_for("tiktok.gift").len(), 0);
    drop_harness(harness);
}

#[tokio::test]
async fn full_global_slots_shed_load_without_calling_plugins() {
    let manifest = processor_manifest(
        "busy",
        &["events.enrich"],
        json!([{"id": "busy.analyze", "title": {"default": "Busy"}}]),
    );
    let harness = make_harness(
        &[("busy", manifest)],
        scripted(|_| panic!("overloaded events must not reach plugins")),
    );
    // Hold every global slot so the next event finds the queue full.
    let _held = Arc::clone(&harness.slots)
        .acquire_many_owned(MAX_TOTAL_PROCESSOR_SLOTS as u32)
        .await
        .unwrap();
    let event = chat_event();
    let enriched = enrich(&harness, event.clone()).await;
    assert_eq!(raw_fields(&enriched), raw_fields(&event));
    assert_eq!(enriched["intel"]["processing"]["status"], "degraded");
    let metrics = harness.metrics.lock().unwrap();
    let metric = &metrics[&ProcessorKey::new("busy", "busy.analyze")];
    assert_eq!(metric.skipped_overloaded, 1);
    assert_eq!(metric.failures, 0);
    drop(metrics);
    // Overload is host backpressure: it must not trip the plugin circuit.
    assert!(processor_retry_allowed(
        &harness.health,
        &ProcessorKey::new("busy", "busy.analyze")
    ));
    drop_harness(harness);
}

#[tokio::test]
async fn failing_processor_does_not_circuit_break_its_sibling() {
    let manifest = processor_manifest(
        "pair",
        &["events.enrich"],
        json!([
            {"id": "pair.flaky", "title": {"default": "Flaky"}, "timeoutMs": 5},
            {"id": "pair.steady", "title": {"default": "Steady"}, "timeoutMs": 2000},
        ]),
    );
    let steady_calls = Arc::new(AtomicU64::new(0));
    let steady_for_handler = Arc::clone(&steady_calls);
    let harness = make_harness(
        &[("pair", manifest)],
        scripted(move |request| {
            let call: tiktools_plugin_sdk::PluginCall = serde_json::from_slice(request).unwrap();
            let request = call.into_enrich().unwrap();
            if request.processor_id == "pair.flaky" {
                // A timeout trips the circuit without retiring the shared
                // worker, so the sibling outcome stays deterministic. The
                // sleep fits well inside the sibling's longer deadline.
                std::thread::sleep(Duration::from_millis(100));
                return Err(PluginLoaderError::Runtime("boom".to_owned()));
            }
            steady_for_handler.fetch_add(1, Ordering::AcqRel);
            Ok(enrich_response(
                json!({"comment": {"normalized": "steady"}}),
                json!({}),
            ))
        }),
    );
    let flaky = ProcessorKey::new("pair", "pair.flaky");
    let steady = ProcessorKey::new("pair", "pair.steady");

    let enriched = enrich(&harness, chat_event()).await;
    assert_eq!(enriched["intel"]["comment"]["normalized"], "steady");
    assert_eq!(enriched["intel"]["processing"]["status"], "degraded");
    assert!(!processor_retry_allowed(&harness.health, &flaky));

    // The sibling keeps enriching while the flaky processor backs off.
    let enriched = enrich(&harness, chat_event()).await;
    assert_eq!(enriched["intel"]["comment"]["normalized"], "steady");
    assert_eq!(steady_calls.load(Ordering::Acquire), 2);
    assert!(processor_retry_allowed(&harness.health, &steady));
    let metrics = harness.metrics.lock().unwrap();
    assert_eq!(metrics[&flaky].skipped_circuit_open, 1);
    assert_eq!(metrics[&steady].successes, 2);
    drop(metrics);
    drop_harness(harness);
}
