//! Real packaged controller UI → Wry bridge → durable native journal → TLS host.
use super::*;
use std::{collections::BTreeMap, sync::Mutex as StdMutex};
use tiktools_client::remote::{
    controller::{LocalCommandOutcome, WorkflowCommandOperation},
    profiles::CredentialStore,
    ClientKind, PairingPoll, RemoteEndpoint, SecretString, TlsTrust,
};
use tiktools_core::{
    collaboration::{Capability, Configuration, Grant, InviteCreate, ResourceKind},
    AppCore, HostEmitter, NoopMediaHost,
};

#[derive(Default)]
struct Store(StdMutex<BTreeMap<String, SecretString>>);
impl CredentialStore for Store {
    fn load(&self, k: &str) -> Result<Option<SecretString>, ClientError> {
        Ok(self.0.lock().unwrap().get(k).cloned())
    }
    fn save(&self, k: &str, v: &SecretString) -> Result<(), ClientError> {
        self.0.lock().unwrap().insert(k.into(), v.clone());
        Ok(())
    }
    fn delete(&self, k: &str) -> Result<(), ClientError> {
        self.0.lock().unwrap().remove(k);
        Ok(())
    }
}
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: tiktools_core::ipc::messages::HostMessage) {}
}
struct CommandDriver {
    app: ControllerWindow,
    phase: u8,
    advanced: Arc<AtomicBool>,
    probing: Arc<AtomicBool>,
    deadline: Instant,
    origin: String,
    minimum_generation: u64,
    diagnostic: Arc<StdMutex<String>>,
}
impl CommandDriver {
    fn probe(&self, source: String) {
        if self.probing.swap(true, Ordering::AcqRel) {
            return;
        }
        let advanced = self.advanced.clone();
        let probing = self.probing.clone();
        let diagnostic = self.diagnostic.clone();
        let source=format!("(()=>{{try{{return {{ok:({source}),text:document.body.innerText.slice(0,1800)}};}}catch(e){{return {{ok:false,error:String(e)}};}}}})()");
        self.app
            .webview
            .as_ref()
            .unwrap()
            .evaluate_script_with_callback(&source, move |value| {
                let value: serde_json::Value =
                    serde_json::from_str(&value).unwrap_or(serde_json::Value::Null);
                *diagnostic.lock().unwrap() = value.to_string();
                if value["ok"] == true {
                    advanced.store(true, Ordering::Release);
                }
                probing.store(false, Ordering::Release);
            })
            .unwrap();
    }
    fn confirmed(&self, operation: WorkflowCommandOperation, revision: &str) -> bool {
        let s = self.app.controller.status();
        !s.command_busy
            && s.command_journal.is_some_and(|j| {
                j.pending.is_none()
                    && j.recent.first().is_some_and(|r| {
                        r.outcome == LocalCommandOutcome::Succeeded
                            && r.intent.operation == operation
                            && r.revision.as_deref() == Some(revision)
                    })
            })
    }
}
impl ApplicationHandler for CommandDriver {
    fn resumed(&mut self, e: &ActiveEventLoop) {
        self.app.resumed(e);
    }
    fn window_event(&mut self, e: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        self.app.window_event(e, id, event);
    }
    fn about_to_wait(&mut self, e: &ActiveEventLoop) {
        self.app.about_to_wait(e);
        if let Some(w) = &self.app.window {
            w.set_visible(false);
        }
        assert!(
            Instant::now() < self.deadline,
            "packaged command UI timed out at phase {} (generation {}, ready {}, native {}): {}",
            self.phase,
            self.app.generation.load(Ordering::Acquire),
            self.app.ready.load(Ordering::Acquire),
            self.app.controller.status().phase,
            self.diagnostic.lock().unwrap()
        );
        if self.app.webview.is_none() {
            return;
        }
        if self.app.generation.load(Ordering::Acquire) < self.minimum_generation
            || self.app.ready.load(Ordering::Acquire) != self.app.generation.load(Ordering::Acquire)
        {
            return;
        }
        if self.advanced.swap(false, Ordering::AcqRel) {
            self.phase += 1;
        }
        let origin = serde_json::to_string(&self.origin).unwrap();
        match self.phase {
            20=>self.probe(r#"(()=>{const root=document.querySelector('.controller');if(!root||document.querySelector('.app-shell'))return false;const nav=root.querySelector('nav button');if(!nav)return false;nav.click();const i=[...root.querySelectorAll('textarea')].find(i=>['Local draft','Borrador local'].includes(i.getAttribute('aria-label')));if(!i||i.disabled)return false;if(i.value!=='native persisted fixture draft'){i.value='native persisted fixture draft';i.dispatchEvent(new Event('input',{bubbles:true}));}const b=[...root.querySelectorAll('button')].find(b=>['Save draft on this PC','Guardar borrador en esta PC'].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;})()"#.into()),
            21=>self.probe(format!(r#"(()=>{{const d=document.querySelector('dialog[open]');if(!d||!d.textContent.includes({origin})||!d.textContent.includes('welcome')||!(d.textContent.includes('sensitive values')||d.textContent.includes('valores sensibles')))return false;const b=[...d.querySelectorAll('button')].find(b=>['Save draft on this PC','Guardar borrador en esta PC'].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;}})()"#)),
            22=>self.probe(r#"(()=>{const t=document.querySelector('.controller__draftStorage')?.textContent;return !!t&&(t.includes('This text matches the saved copy on this PC.')||t.includes('Este texto coincide con la copia guardada en esta PC.'));})()"#.into()),
            23=>{self.minimum_generation=self.app.generation.load(Ordering::Acquire)+1;self.app.webview.as_ref().unwrap().reload().unwrap();self.phase=24;},
            24=>self.probe(r#"(()=>{const root=document.querySelector('.controller');const nav=root?.querySelector('nav button');if(!nav||document.querySelector('.app-shell'))return false;nav.click();const i=[...root.querySelectorAll('textarea')].find(i=>['Local draft','Borrador local'].includes(i.getAttribute('aria-label')));return !!i&&!i.disabled&&i.value==='native persisted fixture draft';})()"#.into()),
            25=>self.phase=0,
            0=>self.probe(r#"(()=>{if(document.querySelector('.app-shell'))return false;const nav=document.querySelector('.controller nav button');if(!nav)return false;nav.click();const b=[...document.querySelectorAll('.controller button')].find(b=>['Disable workflow','Desactivar workflow'].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;})()"#.into()),
            1=>self.probe(format!(r#"(()=>{{const d=document.querySelector('dialog[open]');if(!d||!d.textContent.includes({origin})||!(d.textContent.includes('Revision 1')||d.textContent.includes('Revisión 1')))return false;const b=[...d.querySelectorAll('button')].find(b=>['Disable workflow','Desactivar workflow'].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;}})()"#)),
            2=>{if self.confirmed(WorkflowCommandOperation::Disable,"2"){self.minimum_generation=self.app.generation.load(Ordering::Acquire)+1;self.app.webview.as_ref().unwrap().reload().unwrap();self.phase=3;}},
            3=>self.probe(r#"(()=>{const root=document.querySelector('.controller');if(!root||!(root.textContent.includes('Confirmed by host')||root.textContent.includes('Confirmado por el anfitrión'))||document.querySelector('.app-shell'))return false;const nav=[...root.querySelectorAll('nav button')].find(b=>b.textContent.trim().endsWith('· 2'));if(!nav)return false;nav.click();const b=[...root.querySelectorAll('button')].find(b=>['Delete workflow','Eliminar workflow'].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;})()"#.into()),
            4=>self.probe(format!(r#"(()=>{{const d=document.querySelector('dialog[open]');if(!d||!d.textContent.includes({origin})||!(d.textContent.includes('Revision 2')||d.textContent.includes('Revisión 2')))return false;const b=[...d.querySelectorAll('button')].find(b=>['Delete workflow','Eliminar workflow'].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;}})()"#)),
            5=>{if self.confirmed(WorkflowCommandOperation::Delete,"3"){self.probe(r#"(()=>!!document.querySelector('.controller')&&!document.querySelector('.app-shell')&&(document.querySelector('.controller').textContent.includes('No workflows are available for this access.')||document.querySelector('.controller').textContent.includes('No hay workflows disponibles para este acceso.')))()"#.into());}},
            6=>self.probe(r#"(()=>{const f=document.querySelector('.controller__pointSearch');const i=f?.querySelector('input');if(!i)return false;i.value='alice';i.dispatchEvent(new Event('input',{bubbles:true}));f.requestSubmit();return true;})()"#.into()),
            7=>self.probe(r#"(()=>{const target=document.querySelector('.controller__pointTarget');const i=target?.querySelector('input[type=number]');if(!i||i.disabled||!target.textContent.includes('5 Points'))return false;i.value='-3';i.dispatchEvent(new Event('input',{bubbles:true}));setTimeout(()=>target.querySelector('form').requestSubmit(),0);return true;})()"#.into()),
            8=>self.probe(format!(r#"(()=>{{const d=document.querySelector('dialog[open]');if(!d||!d.textContent.includes({origin})||!d.textContent.includes('alice')||!d.textContent.includes('-3 Points'))return false;const b=[...d.querySelectorAll('button')].find(b=>['Adjust points','Ajustar puntos'].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;}})()"#)),
            9=>{if self.confirmed(WorkflowCommandOperation::AdjustPoints,"2"){self.minimum_generation=self.app.generation.load(Ordering::Acquire)+1;self.app.webview.as_ref().unwrap().reload().unwrap();self.phase=10;}},
            10=>self.probe(r#"(()=>{const root=document.querySelector('.controller');if(!root||document.querySelector('.app-shell'))return false;return root.textContent.includes('alice')&&root.textContent.includes('-3 Points')&&root.textContent.includes('2 Points')&&(root.textContent.includes('Confirmed by host')||root.textContent.includes('Confirmado por el anfitrión'));})()"#.into()),
            11=>self.probe(r#"(()=>{const root=document.querySelector('.controller');const nav=[...root.querySelectorAll('nav button')].find(b=>b.textContent.includes('welcome'));if(!nav)return false;nav.click();const i=[...root.querySelectorAll('textarea')].find(i=>['Local draft','Borrador local'].includes(i.getAttribute('aria-label')));if(!i||i.disabled||i.value!=='native persisted fixture draft')return false;const b=[...root.querySelectorAll('button')].find(b=>['Discard draft','Descartar borrador'].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;})()"#.into()),
            12=>self.probe(r#"(()=>{const root=document.querySelector('.controller');return !!root&&!root.querySelector('nav button')&&!root.querySelector('textarea');})()"#.into()),
            13=>self.probe(r#"(()=>{const root=document.querySelector('.automation-board');if(!root||document.querySelector('.app-shell')||document.body.textContent.includes('native-secret-payload'))return false;const b=[...root.querySelectorAll('button')].find(b=>['Disable event','Desactivar evento'].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;})()"#.into()),
            14=>self.probe(format!(r#"(()=>{{const d=document.querySelector('dialog[open]');if(!d||!d.textContent.includes({origin})||!d.textContent.includes('welcome')||!(d.textContent.includes('Revision 1')||d.textContent.includes('Revisión 1')))return false;const b=[...d.querySelectorAll('button')].find(b=>['Disable event','Desactivar evento'].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;}})()"#)),
            15=>{if self.confirmed(WorkflowCommandOperation::DisableEvent,"2"){self.minimum_generation=self.app.generation.load(Ordering::Acquire)+1;self.app.webview.as_ref().unwrap().reload().unwrap();self.phase=16;}},
            16=>self.probe(r#"(()=>{const root=document.querySelector('.automation-board');if(!root||document.querySelector('.app-shell')||!(root.textContent.includes('Revision 2')||root.textContent.includes('Revisión 2')))return false;const b=[...root.querySelectorAll('button')].find(b=>['Delete event','Eliminar evento'].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;})()"#.into()),
            17=>self.probe(format!(r#"(()=>{{const d=document.querySelector('dialog[open]');if(!d||!d.textContent.includes({origin})||!(d.textContent.includes('Revision 2')||d.textContent.includes('Revisión 2')))return false;const b=[...d.querySelectorAll('button')].find(b=>['Delete event','Eliminar evento'].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;}})()"#)),
            18=>{if self.confirmed(WorkflowCommandOperation::DeleteEvent,"3"){self.probe(r#"(()=>{const root=document.querySelector('.automation-board');return !!root&&root.querySelectorAll('li').length===1&&(root.textContent.includes('Action')||root.textContent.includes('Acción'))&&!root.querySelector('button')&&!document.querySelector('.app-shell');})()"#.into());}},
            19=>self.phase=30,
            30=>self.probe(r#"(()=>{const r=document.querySelector('.proposals'),f=r?.querySelector('form'),i=f?.querySelector('input');if(!i||i.disabled)return false;i.value='proposal';i.dispatchEvent(new Event('input',{bubbles:true}));f.requestSubmit();return true;})()"#.into()),
            31=>self.probe(r#"(()=>{const r=document.querySelector('.proposals'),t=r?.querySelector('textarea');if(!t||t.disabled)return false;const b=JSON.parse(t.value);b.name='Native proposed action';b.config={message:'native-proposal-secret'};t.value=JSON.stringify(b,null,2);t.dispatchEvent(new Event('input',{bubbles:true}));const i=[...r.querySelectorAll('button')].find(b=>['Inspect current status','Consultar estado actual'].includes(b.textContent.trim())&&!b.disabled);if(!i)return false;i.click();return true;})()"#.into()),
            32=>self.proposal_button("Save proposal on this PC","Guardar propuesta en esta PC"),
            33=>self.proposal_confirmation("Save proposal on this PC","Guardar propuesta en esta PC","native-proposal-secret"),
            34=>self.proposal_button("Send proposal for review","Enviar propuesta para revisión"),
            35=>self.proposal_confirmation("Send proposal for review","Enviar propuesta para revisión","native-proposal-secret"),
            36=>{if self.confirmed(WorkflowCommandOperation::CreateActionDraft,"1"){self.minimum_generation=self.app.generation.load(Ordering::Acquire)+1;self.app.webview.as_ref().unwrap().reload().unwrap();self.phase=37;}},
            37=>self.probe(r#"(()=>{const r=document.querySelector('.proposals'),d=r?.querySelector('details');if(!d||document.querySelector('.app-shell'))return false;d.open=true;const b=d.querySelector('button');if(!b||b.disabled)return false;b.click();return true;})()"#.into()),
            38=>self.probe(r#"(()=>{const r=document.querySelector('.proposals'),t=r?.querySelector('textarea');if(!t||t.disabled||!t.value.includes('native-proposal-secret'))return false;const b=[...r.querySelectorAll('button')].find(b=>['Inspect current status','Consultar estado actual'].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;})()"#.into()),
            39=>self.proposal_button("Use this status as base","Usar este estado como base"),
            40=>self.proposal_confirmation("Use this status as base","Usar este estado como base","native-proposal-secret"),
            41=>self.probe(r#"(()=>{const r=document.querySelector('.proposals'),t=r?.querySelector('textarea');if(!t||t.disabled||!(r.textContent.includes('Base revision 1')||r.textContent.includes('Revisión base 1')))return false;const b=JSON.parse(t.value);b.name='Native replaced action';t.value=JSON.stringify(b,null,2);t.dispatchEvent(new Event('input',{bubbles:true}));return true;})()"#.into()),
            42=>self.proposal_button("Save proposal on this PC","Guardar propuesta en esta PC"),
            43=>self.proposal_confirmation("Save proposal on this PC","Guardar propuesta en esta PC","Native replaced action"),
            44=>self.proposal_button("Send proposal for review","Enviar propuesta para revisión"),
            45=>self.proposal_confirmation("Send proposal for review","Enviar propuesta para revisión","Native replaced action"),
            46=>{if self.confirmed(WorkflowCommandOperation::UpdateActionDraft,"2"){self.minimum_generation=self.app.generation.load(Ordering::Acquire)+1;self.app.webview.as_ref().unwrap().reload().unwrap();self.phase=47;}},
            47=>self.probe(r#"(()=>{const r=document.querySelector('.controller'),j=r?.querySelector('.controller__journal');return !!j&&!document.querySelector('.app-shell')&&j.textContent.includes('proposal')&&(j.textContent.includes('Propose action replacement')||j.textContent.includes('Proponer reemplazo de acción'))&&![...Object.keys(localStorage),...Object.keys(sessionStorage)].some(k=>/proposal|native-proposal-secret/.test(k))&&![...Object.values(localStorage),...Object.values(sessionStorage)].some(v=>v.includes('native-proposal-secret'));})()"#.into()),
            48=>e.exit(),_=>unreachable!(),
        }
    }
}
impl CommandDriver {
    fn proposal_button(&self, en: &str, es: &str) {
        self.probe(format!(r#"(()=>{{const r=document.querySelector('.proposals');const b=[...r?.querySelectorAll('button')??[]].find(b=>[{en:?},{es:?}].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;}})()"#));
    }
    fn proposal_confirmation(&self, en: &str, es: &str, source: &str) {
        let origin = serde_json::to_string(&self.origin).unwrap();
        self.probe(format!(r#"(()=>{{const d=document.querySelector('dialog[open]');if(!d||!d.textContent.includes({origin})||!d.textContent.includes({source:?})||!d.textContent.includes('proposal'))return false;const b=[...d.querySelectorAll('button')].find(b=>[{en:?},{es:?}].includes(b.textContent.trim())&&!b.disabled);if(!b)return false;b.click();return true;}})()"#));
    }
}
#[test]
#[ignore = "requires a Linux display and built dist/web; run alone"]
fn packaged_controller_confirms_tls_commands_and_keeps_history_after_reload() {
    std::env::set_var("TIKTOOLS_CLOUD_ORIGIN", "");
    let temp = tempfile::tempdir().unwrap();
    let host = temp.path().join("host");
    let client = temp.path().join("client");
    let paths = tiktools_core::paths::AppPaths {
        data: host.join("data"),
        plugins: host.join("plugins"),
        plugin_data: host.join("plugin-data"),
        builtin_plugins: host.join("builtins"),
        development_plugins: None,
        logs: host.join("logs"),
        temp: host.join("temp"),
        root: host,
    };
    paths.ensure_directories().unwrap();
    // Keep this host synthetic even when the checkout contains legacy data.
    for path in [
        paths.points_database(),
        paths.automation_database(),
        paths.analytics_database(),
    ] {
        std::fs::File::create(path).unwrap();
    }
    let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
    let (core, gateway, manager, origin) = runtime.block_on(async {
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths,
        ));
        let api = Arc::new(tiktools_control_api::ControlApi::new(core.clone()));
        core.workflow_save(json!({"id":"welcome","name":"Hosted workflow","enabled":true}))
            .unwrap();
        core.points_adjust("alice", 5.0).unwrap();
        for kind in [tiktools_core::control::AutomationKind::Event,tiktools_core::control::AutomationKind::Action] {
            core.automation_create(kind,json!({"id":"welcome","name":"Hosted behavior","enabled":true,"trigger":"tiktok.chat","typeId":"core.log","actionIds":["welcome"],"config":{"message":"native-secret-payload"}})).unwrap();
        }
        core.automation_create(tiktools_core::control::AutomationKind::Event,json!({"id":"hidden","name":"Hidden behavior","enabled":true})).unwrap();
        let service = core.collaboration().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let origin = format!("https://{address}");
        let pair = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
        let cert = temp.path().join("cert.pem");
        let key = temp.path().join("key.pem");
        std::fs::write(&cert, pair.cert.pem()).unwrap();
        std::fs::write(&key, pair.signing_key.serialize_pem()).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        service
            .configure(Configuration {
                enabled: true,
                bind_address: address.to_string(),
                advertised_origin: Some(origin.clone()),
                tls_certificate: Some(cert.to_string_lossy().into()),
                tls_private_key: Some(key.to_string_lossy().into()),
                ..Configuration::default()
            })
            .unwrap();
        let gateway = tiktools_remote::start_on(api, listener).await.unwrap();
        let pin = ring::digest::digest(&ring::digest::SHA256, pair.cert.der())
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let endpoint = RemoteEndpoint::new(&origin, TlsTrust::CertificateSha256 { sha256: pin })
            .await
            .unwrap();
        let invite = service
            .create_invitation(InviteCreate {
                grant: Grant {
                    capabilities: [
                        Capability::WorkflowsRead,
                        Capability::WorkflowsToggle,
                        Capability::WorkflowsEdit,
                        Capability::CommandsRead,
                        Capability::PointsRead,
                        Capability::PointsAdjust,
                        Capability::AutomationRead,
                        Capability::AutomationToggle,
                        Capability::AutomationEdit,
                    ]
                    .into(),
                    resources: [
                        (ResourceKind::Workflow, ["welcome".into()].into()),
                        (ResourceKind::Viewer, ["alice".into()].into()),
                        (ResourceKind::Automation,["event:welcome".into(),"action:welcome".into(),"action:proposal".into()].into()),
                    ]
                    .into(),
                    ..Grant::default()
                },
            })
            .unwrap();
        let mut pending = endpoint
            .begin_pairing(
                &invite.invite_id,
                &SecretString::from(invite.secret),
                "Controller test",
                ClientKind::Desktop,
            )
            .await
            .unwrap();
        service.decide_request(&pending.request_id, true).unwrap();
        let credentials = match pending.poll().await.unwrap() {
            PairingPoll::Approved(c) => c,
            _ => panic!("fixture approval failed"),
        };
        let manager =
            ProfileManager::new(client.join("profiles"), Arc::new(Store::default())).unwrap();
        manager
            .install("desk", &endpoint, credentials, ClientKind::Desktop)
            .await
            .unwrap();
        (core, gateway, manager, origin)
    });
    let controller = Arc::new(Controller::new(manager));
    runtime.block_on(controller.connect("desk")).unwrap();
    platform::initialize().unwrap();
    let mut builder = EventLoop::<()>::with_user_event();
    builder.with_any_thread(true);
    platform::configure_event_loop(&mut builder);
    let event_loop = builder.build().unwrap();
    let (tx, rx) = mpsc::channel(32);
    let changed = controller.subscribe();
    let app = ControllerWindow {
        source: FrontendSource::EmbeddedAssets {
            root: Arc::new(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../dist/web")
                    .canonicalize()
                    .unwrap(),
            ),
        },
        controller: controller.clone(),
        runtime: runtime.handle().clone(),
        profile: None,
        window: None,
        webview: None,
        tx,
        rx,
        changed,
        generation: Arc::new(AtomicU64::new(0)),
        ready: Arc::new(AtomicU64::new(u64::MAX)),
        deadline: None,
        observed_generation: 0,
        log_path: client.join("controller.log"),
    };
    let mut driver = CommandDriver {
        app,
        phase: 20,
        advanced: Arc::new(AtomicBool::new(false)),
        probing: Arc::new(AtomicBool::new(false)),
        deadline: Instant::now() + Duration::from_secs(100),
        origin,
        minimum_generation: 1,
        diagnostic: Arc::new(StdMutex::new("not probed".into())),
    };
    event_loop.run_app(&mut driver).unwrap();
    assert_eq!(driver.phase, 48);
    assert!(driver.app.generation.load(Ordering::Acquire) >= 4);
    assert!(core.workflow_get("welcome").is_err());
    assert_eq!(
        core.workflow_get_versioned("welcome")
            .unwrap()
            .resource_versions[0]
            .revision,
        "3"
    );
    assert_eq!(core.points_viewer("alice").unwrap()["points"], 2.0);
    let status = controller.status();
    let draft = runtime
        .block_on(controller.manage(
            "controller.drafts.get",
            json!({"epoch":status.epoch,"viewRevision":status.view_revision,"id":"welcome"}),
        ))
        .unwrap();
    assert!(draft["draft"].is_null());
    assert_eq!(draft["storeRevision"], "2");
    let journal = controller.status().command_journal.unwrap();
    assert_eq!(journal.recent.len(), 7);
    assert_eq!(
        journal.recent[4]
            .points_result
            .as_ref()
            .unwrap()
            .total_points,
        2.0
    );
    assert!(core
        .automation_get(tiktools_core::control::AutomationKind::Event, "welcome")
        .is_err());
    assert_eq!(
        core.automation_get_versioned(tiktools_core::control::AutomationKind::Event, "welcome")
            .unwrap()
            .resource_versions[0]
            .revision,
        "3"
    );
    assert_eq!(
        core.automation_get_versioned(tiktools_core::control::AutomationKind::Action, "welcome")
            .unwrap()
            .resource_versions[0]
            .revision,
        "1"
    );
    assert!(core.automation_runs().is_empty());
    let proposal = core
        .automation_get(tiktools_core::control::AutomationKind::Action, "proposal")
        .unwrap();
    assert_eq!(proposal["name"], "Native replaced action");
    assert_eq!(proposal["enabled"], false);
    assert!(serde_json::to_value(
        core.automation_authority(tiktools_core::control::AutomationKind::Action, "proposal")
            .unwrap()
    )
    .unwrap()
    .to_string()
    .contains("delegated"));
    for entry in std::fs::read_dir(&client).unwrap() {
        assert_eq!(entry.unwrap().file_name(), "profiles");
    }
    runtime.block_on(async {
        controller.disconnect().await;
        gateway.shutdown().await.unwrap();
        core.shutdown().await;
    });
}
