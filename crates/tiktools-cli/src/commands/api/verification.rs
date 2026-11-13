//! Read-only registry coverage and bounded smoke verification.
use super::*;

/// Read-only smoke calls: every entry must succeed with fixed safe
/// params on any healthy host. Coverage of mutating methods belongs to
/// the host test suite, never to a CLI that agents run against live data.
const SMOKE_CALLS: &[(&str, &str)] = &[
    ("system.ping", "{}"),
    ("system.info", "{}"),
    ("system.health", "{}"),
    ("system.snapshot", "{}"),
    ("rpc.discover", "{}"),
    ("plugins.list", "{}"),
    ("plugins.diagnostics", "{}"),
    ("automation.list", r#"{"kind":"all"}"#),
    ("automation.context", "{}"),
    ("automation.runs", "{}"),
    ("automation.nodes.list", "{}"),
    ("points.config.get", "{}"),
    ("points.leaderboard", "{}"),
    ("workflows.list", "{}"),
    ("processors.list", "{}"),
    ("processors.status", "{}"),
    ("creators.recent", "{}"),
    ("gifts.list", "{}"),
    ("gifts.debug", "{}"),
    ("live.status", "{}"),
    ("app.state.get", "{}"),
    ("globals.list", "{}"),
    ("globals.typed.list", "{}"),
];

/// Default per-call budget for verification. The host budget is 150s;
/// verification calls are all cheap reads and fail fast instead.
const VERIFY_TIMEOUT_SECS: u64 = 30;

pub(super) async fn execute_verify(
    client: &TikToolsClient,
    coverage: bool,
    smoke: bool,
    timeout_secs: Option<u64>,
    json: Option<bool>,
) -> Result<Output, ClientError> {
    let budget = Duration::from_secs(timeout_secs.unwrap_or(VERIFY_TIMEOUT_SECS));
    let mut report = serde_json::Map::new();
    let mut ok = true;
    if coverage {
        let (coverage_ok, value) = verify_coverage(client, budget).await?;
        ok &= coverage_ok;
        report.insert("coverage".to_owned(), value);
    }
    if smoke {
        let (smoke_ok, value) = verify_smoke(client, budget).await;
        ok &= smoke_ok;
        report.insert("smoke".to_owned(), value);
    }
    report.insert("ok".to_owned(), Value::Bool(ok));
    Ok(Output::new("api-verify", Value::Object(report))
        .json_override(json)
        .exit_code(i32::from(!ok)))
}

/// Compares the host's live registry against the typed methods compiled
/// into this CLI. The parity test pins the two at build time; this pins
/// them at deploy time, catching CLI/host version skew.
async fn verify_coverage(
    client: &TikToolsClient,
    budget: Duration,
) -> Result<(bool, Value), ClientError> {
    let discovered: tiktools_control_api::modules::rpc::DiscoverResult =
        call_with_budget(client, "rpc.discover", json!({}), budget).await?;
    let live: std::collections::BTreeSet<&str> = discovered
        .methods
        .iter()
        .map(|meta| meta.name.as_str())
        .collect();
    let compiled: std::collections::BTreeSet<&str> =
        TikToolsClient::covered_methods().into_iter().collect();
    let missing: Vec<&str> = live.difference(&compiled).copied().collect();
    let extra: Vec<&str> = compiled.difference(&live).copied().collect();
    let ok = missing.is_empty() && extra.is_empty();
    Ok((
        ok,
        json!({
            "ok": ok,
            "live": live.len(),
            "compiled": compiled.len(),
            "missing": missing,
            "extra": extra,
        }),
    ))
}

/// Runs every read-only smoke call, recording per-method pass/fail. One
/// failing method never aborts the run: agents get the whole picture.
async fn verify_smoke(client: &TikToolsClient, budget: Duration) -> (bool, Value) {
    let mut results = Vec::with_capacity(SMOKE_CALLS.len());
    let mut failed = 0;
    for (method, params) in SMOKE_CALLS {
        let params: Value = serde_json::from_str(params).unwrap_or(Value::Null);
        match call_with_budget::<Value>(client, method, params, budget).await {
            Ok(_) => results.push(json!({"method": method, "ok": true})),
            Err(error) => {
                failed += 1;
                results.push(json!({
                    "method": method,
                    "ok": false,
                    "code": error.code,
                    "message": error.message,
                }));
            }
        }
    }
    let ok = failed == 0;
    (
        ok,
        json!({
            "ok": ok,
            "passed": results.len() - failed,
            "failed": failed,
            "results": results,
        }),
    )
}

async fn call_with_budget<R>(
    client: &TikToolsClient,
    method: &str,
    params: Value,
    budget: Duration,
) -> Result<R, ClientError>
where
    R: serde::de::DeserializeOwned,
{
    match tokio::time::timeout(budget, client.call(method, params)).await {
        Ok(outcome) => outcome,
        Err(_) => Err(ClientError::new(
            "timeout",
            format!("{method} exceeded the verification budget"),
        )),
    }
}
