//! Native controller projection. Never constructs a host or interprets legacy IPC.
//! WebViews receive nonsecret state; credentials stay in ProfileManager.
use super::{profiles::ProfileManager, *};
use crate::{CallResult, RequestMetadata};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::Duration,
};
use tiktools_collaboration::{
    Capability, ResourceKind, ResourceVersion, SnapshotPage, SNAPSHOT_BYTES,
};
use tiktools_control_api::modules::collaboration::HelloResult;
use tokio::sync::{watch, Mutex};
mod automations;
mod commands;
mod drafts;
mod live_status;
mod points;
mod proposals;
pub use automations::AutomationRow;
pub use commands::{AutomationCommand, PointsCommand, WorkflowCommand};
use profiles::commands::Journal;
pub use profiles::commands::Operation as CommandOperation;
pub use profiles::commands::{
    Binding as CommandBinding, Intent as PendingCommand, JournalView,
    Operation as WorkflowCommandOperation, Outcome as LocalCommandOutcome, PointsResult,
    Record as CommandRecord,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerStatus {
    pub drafts_supported: bool,
    pub proposals_supported: bool,
    pub epoch: String,
    pub view_revision: String,
    pub points_revision: String,
    pub automation_revision: String,
    pub points_loading: bool,
    pub points_error_code: Option<String>,
    pub phase: String,
    pub profile: Option<String>,
    pub origin: Option<String>,
    pub hello: Option<HelloResult>,
    pub error_code: Option<String>,
    pub pairing_request_id: Option<String>,
    pub command_journal: Option<JournalView>,
    pub command_busy: bool,
    pub command_error_code: Option<String>,
}
#[derive(Clone, Serialize)]
pub struct WorkflowRow {
    pub graph: Value,
    pub version: ResourceVersion,
}
struct View {
    status: ControllerStatus,
    remote: Option<RemoteClient>,
    rows: BTreeMap<String, WorkflowRow>,
    journal: Option<Arc<Journal>>,
    points: points::State,
    automation: automations::State,
    proposal_inspection: Option<proposals::Inspection>,
}
fn journal_projection(v: &View) -> Option<JournalView> {
    let mut journal = v.journal.as_ref()?.view();
    let hello = v.status.hello.as_ref().filter(|h| {
        v.status.phase == "ready"
            && v.remote
                .as_ref()
                .is_some_and(|r| !r.is_closed() && !r.requires_resync())
            && journal.binding.host_id == h.host_id
            && journal.binding.principal_id == h.principal_id
            && journal.binding.session_id == h.session_id
            && Some(journal.binding.origin.as_str()) == v.status.origin.as_deref()
    });
    for record in &mut journal.recent {
        let allowed = hello.is_some_and(|h| {
            h.capabilities.contains(&Capability::PointsRead)
                && !h.resource_restrictions.contains_key(&ResourceKind::Live)
                && h.resource_restrictions
                    .get(&ResourceKind::Viewer)
                    .is_none_or(|ids| ids.contains(&record.intent.resource_id))
        });
        if !allowed {
            record.points_result = None;
        }
    }
    Some(journal)
}
struct Shared {
    view: RwLock<View>,
    changed: watch::Sender<u64>,
}
impl Shared {
    fn change(&self, epoch: &str, apply: impl FnOnce(&mut View)) {
        let mut v = self.view.write().unwrap_or_else(|p| p.into_inner());
        if v.status.epoch != epoch {
            return;
        }
        apply(&mut v);
        let revision = v
            .status
            .view_revision
            .parse::<u64>()
            .unwrap_or(0)
            .saturating_add(1);
        v.status.view_revision = revision.to_string();
        self.changed.send_modify(|n| *n = n.wrapping_add(1));
    }
    fn phase(&self, epoch: &str, phase: &str, error: Option<&ClientError>) {
        self.change(epoch, |v| {
            v.status.phase = phase.into();
            v.status.error_code = error.map(|e| e.code.clone());
        });
    }
    fn ready(
        &self,
        epoch: &str,
        remote: &RemoteClient,
        baseline: Option<&SnapshotPage>,
    ) -> Result<(), ClientError> {
        let mut result = Ok(());
        self.change(epoch, |v| {
            if !automations::readable(remote) {
                automations::clear(v);
            }
            if let Some(page) = baseline {
                if let Err(error) = automations::baseline(v, remote, page) {
                    result = Err(error);
                    return;
                }
                let versions: BTreeMap<_, _> = page
                    .resource_versions
                    .iter()
                    .filter(|v| v.kind == ResourceKind::Workflow)
                    .map(|v| (v.id.as_str(), v))
                    .collect();
                v.rows = page
                    .workflows
                    .iter()
                    .filter_map(|graph| {
                        let id = graph.get("id")?.as_str()?;
                        let version = versions.get(id)?;
                        Some((
                            id.to_owned(),
                            WorkflowRow {
                                graph: graph.clone(),
                                version: (*version).clone(),
                            },
                        ))
                    })
                    .collect();
            }
            v.remote = Some(remote.clone());
            v.status.hello = Some(remote.hello().clone());
            v.status.phase = "ready".into();
            v.status.pairing_request_id = None;
            v.status.error_code = None;
        });
        result
    }
}
struct Pairing {
    profile: String,
    endpoint: RemoteEndpoint,
    pending: PendingPairing,
}
#[derive(Default)]
struct Lifecycle {
    worker: Option<tokio::task::JoinHandle<()>>,
    pairing: Option<Pairing>,
}
impl Drop for Lifecycle {
    fn drop(&mut self) {
        if let Some(worker) = &self.worker {
            worker.abort();
        }
    }
}
pub struct Controller {
    profiles: ProfileManager,
    shared: Arc<Shared>,
    lifecycle: Mutex<Lifecycle>,
    commands: Arc<Mutex<()>>,
    journals: std::sync::Mutex<BTreeMap<String, std::sync::Weak<Journal>>>,
}
impl Controller {
    pub fn new(profiles: ProfileManager) -> Self {
        let (changed, _) = watch::channel(0);
        Self {
            profiles,
            lifecycle: Mutex::new(Lifecycle::default()),
            commands: Arc::new(Mutex::new(())),
            journals: std::sync::Mutex::new(BTreeMap::new()),
            shared: Arc::new(Shared {
                changed,
                view: RwLock::new(View {
                    status: ControllerStatus {
                        drafts_supported: true,
                        proposals_supported: true,
                        epoch: "0".into(),
                        view_revision: "0".into(),
                        points_revision: "0".into(),
                        automation_revision: "0".into(),
                        points_loading: true,
                        points_error_code: None,
                        phase: "idle".into(),
                        profile: None,
                        origin: None,
                        hello: None,
                        error_code: None,
                        pairing_request_id: None,
                        command_journal: None,
                        command_busy: false,
                        command_error_code: None,
                    },
                    remote: None,
                    rows: BTreeMap::new(),
                    journal: None,
                    points: points::State::default(),
                    automation: automations::State::default(),
                    proposal_inspection: None,
                }),
            }),
        }
    }
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.shared.changed.subscribe()
    }
    pub fn status(&self) -> ControllerStatus {
        let v = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
        let mut status = v.status.clone();
        status.command_journal = journal_projection(&v);
        if status.phase == "ready"
            && v.remote
                .as_ref()
                .is_some_and(|r| r.is_closed() || r.requires_resync())
        {
            status.phase = "reconnecting".into();
        }
        status
    }
    fn reset(&self, lifecycle: &mut Lifecycle) -> String {
        if let Some(worker) = lifecycle.worker.take() {
            worker.abort();
        }
        lifecycle.pairing = None;
        let mut v = self.shared.view.write().unwrap_or_else(|p| p.into_inner());
        if let Some(remote) = v.remote.take() {
            remote.close();
        }
        let epoch = v
            .status
            .epoch
            .parse::<u64>()
            .unwrap_or(0)
            .saturating_add(1)
            .to_string();
        v.rows.clear();
        v.points = points::State::default();
        v.automation = automations::State::default();
        v.journal = None;
        v.status = ControllerStatus {
            drafts_supported: true,
            proposals_supported: true,
            epoch: epoch.clone(),
            view_revision: "0".into(),
            points_revision: "0".into(),
            automation_revision: "0".into(),
            points_loading: true,
            points_error_code: None,
            phase: "idle".into(),
            profile: None,
            origin: None,
            hello: None,
            error_code: None,
            pairing_request_id: None,
            command_journal: None,
            command_busy: false,
            command_error_code: None,
        };
        self.shared.changed.send_modify(|n| *n = n.wrapping_add(1));
        epoch
    }
    pub async fn disconnect(&self) {
        let mut l = self.lifecycle.lock().await;
        self.reset(&mut l);
    }
    pub async fn connect(&self, profile: &str) -> Result<(), ClientError> {
        let mut l = self.lifecycle.lock().await;
        let epoch = self.reset(&mut l);
        self.shared.change(&epoch, |v| {
            v.status.phase = "connecting".into();
            v.status.profile = Some(profile.into());
        });
        let result = async {
            let metadata = self.profiles.get(profile).await?;
            let journal = self.open_journal(&metadata).await?;
            self.shared.change(&epoch, |v| {
                v.status.origin = Some(metadata.origin);
                v.journal = Some(journal);
            });
            let remote = self.profiles.open(profile).await?;
            // Keep a handle during bootstrap so disconnect/shutdown closes it.
            self.shared
                .change(&epoch, |v| v.remote = Some(remote.clone()));
            let recovery = points::stream(&remote).await?;
            self.shared.ready(
                &epoch,
                &remote,
                recovery.as_ref().and_then(|s| s.baseline()),
            )?;
            if let Err(e) = points::refresh(&self.shared, &epoch, &remote).await {
                points::failure(&self.shared, &epoch, &e);
            }
            let shared = self.shared.clone();
            let commands = self.commands.clone();
            commands::schedule_resolve(shared.clone(), commands.clone(), epoch.clone());
            l.worker = Some(tokio::spawn(async move {
                run(shared, commands, epoch, remote, recovery).await;
            }));
            Ok(())
        }
        .await;
        if let Err(error) = &result {
            let epoch = self.status().epoch;
            self.shared.change(&epoch, |v| {
                if let Some(r) = v.remote.take() {
                    r.close();
                }
                v.rows.clear();
                automations::clear(v);
            });
            self.shared.phase(&epoch, "failed", Some(error));
        }
        result
    }
    pub fn workflows(
        &self,
        epoch: &str,
        revision: &str,
        offset: usize,
    ) -> Result<Value, ClientError> {
        let v = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
        if v.status.epoch != epoch || v.status.view_revision != revision {
            return Err(fail("stale_view", "controller view changed; read it again"));
        }
        let Some(remote) = &v.remote else {
            return Err(fail("not_ready", "remote controller is not ready"));
        };
        if v.status.phase != "ready" || remote.is_closed() || remote.requires_resync() {
            return Err(fail("not_ready", "remote controller is recovering"));
        }
        if offset > v.rows.len() {
            return Err(fail("invalid_params", "invalid controller page"));
        }
        let mut rows = Vec::new();
        let mut bytes = 0;
        for row in v.rows.values().skip(offset).take(32) {
            let size = serde_json::to_vec(row).map_err(|_| protocol())?.len();
            if bytes + size > 512 * 1024 {
                break;
            }
            bytes += size;
            rows.push(row.clone());
        }
        if rows.is_empty() && offset < v.rows.len() {
            return Err(fail("too_large", "workflow exceeds controller page budget"));
        }
        let next = offset + rows.len();
        Ok(
            json!({"epoch":epoch,"viewRevision":revision,"rows":rows,"next":if next<v.rows.len(){Some(next)}else{None}}),
        )
    }
    pub async fn forward(
        &self,
        epoch: &str,
        method: &str,
        params: Value,
        meta: Option<RequestMetadata>,
    ) -> Result<CallResult<Value>, ClientError> {
        // Page mutations must use the native durable-intent coordinator. Add
        // completed domain reads explicitly; new methods never bypass it.
        if !matches!(
            method,
            "rpc.discover"
                | "rpc.schema"
                | "collaboration.me"
                | "collaboration.commands.get"
                | "live.status"
                | "live.monitor.status"
                | "ui.feed.backfill"
                | "workflows.list"
                | "workflows.get"
                | "points.info"
                | "points.viewer.get"
                | "points.leaderboard"
        ) || meta.as_ref().is_some_and(|m| {
            m.command_id.is_some() || m.host_instance_id.is_some() || m.expected_revision.is_some()
        }) {
            return Err(fail(
                "forbidden",
                "controller manages its own remote session",
            ));
        }
        let remote = {
            let v = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
            if v.status.epoch != epoch || v.status.phase != "ready" {
                return Err(fail("not_ready", "selected controller is not ready"));
            }
            v.remote.clone().ok_or_else(network)?
        };
        if remote.requires_resync() {
            return Err(fail("not_ready", "controller needs recovery"));
        }
        let result = remote.call_value_with_meta(method, params, meta).await;
        if self.status().epoch != epoch {
            return Err(fail(
                "stale_view",
                "reply belongs to a previous controller selection",
            ));
        }
        result
    }
    /// Native management RPCs. Their result cannot contain access/refresh tokens.
    pub async fn manage(&self, method: &str, params: Value) -> Result<Value, ClientError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Pair {
            profile: String,
            origin: String,
            invitation: String,
            label: String,
            fingerprint: Option<String>,
            #[serde(default)]
            confirm_pin: bool,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Name {
            profile: String,
        }
        match method {
            "controller.drafts.list"=>{
                let p:drafts::Read=parse(params)?;
                if p.id.is_some(){return Err(fail("invalid_params","draft list does not accept an ID"));}
                self.read_drafts(p).await
            },
            "controller.drafts.get"=>{
                let p:drafts::Read=parse(params)?;
                if p.id.is_none()||p.offset!=0{return Err(fail("invalid_params","draft lookup requires one ID"));}
                self.read_drafts(p).await
            },
            "controller.drafts.save"=>self.change_draft(parse(params)?,false).await,
            "controller.drafts.delete"=>self.change_draft(parse(params)?,true).await,
            "controller.points"=>{#[derive(Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]struct P{epoch:String,points_revision:String}let p:P=parse(params)?;self.points(&p.epoch,&p.points_revision)},
            "controller.points.select"=>{#[derive(Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]struct P{epoch:String,unique_id:Option<String>}let p:P=parse(params)?;self.select_points(&p.epoch,p.unique_id)},
            "controller.live.status"=>{#[derive(Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]struct P{epoch:String,connection_id:String}let p:P=parse(params)?;self.live_status(&p.epoch,&p.connection_id).await},
            "controller.live.feed"=>{#[derive(Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]struct P{epoch:String,connection_id:String,after:String,creator_id:Option<String>,stream_id:Option<String>}let p:P=parse(params)?;self.live_feed(&p.epoch,&p.connection_id,&p.after,p.creator_id.as_deref(),p.stream_id.as_deref()).await},
            "controller.workflow.command"=>self.workflow_command(parse(params)?).await,
            "controller.points.command"=>self.points_command(parse(params)?).await,
            "controller.automation.command"=>self.automation_command(parse(params)?).await,
            "controller.proposals.list"=>self.proposal_read(parse(params)?,true).await,
            "controller.proposals.get"=>self.proposal_read(parse(params)?,false).await,
            "controller.proposals.save"=>self.proposal_save(parse(params)?).await,
            "controller.proposals.delete"=>self.proposal_delete(parse(params)?).await,
            "controller.proposals.inspect"=>self.proposal_inspect(parse(params)?).await,
            "controller.proposals.submit"=>self.proposal_command(parse(params)?).await,
            "controller.automations"=>{#[derive(Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]struct P{epoch:String,automation_revision:String,#[serde(default)]offset:usize}let p:P=parse(params)?;self.automations(&p.epoch,&p.automation_revision,p.offset)},
            "controller.command.resolve"=>{
                #[derive(Deserialize)] #[serde(rename_all="camelCase",deny_unknown_fields)] struct Resolve { epoch:String,command_id:String }
                let p:Resolve=parse(params)?; self.resolve_command(p.epoch,p.command_id).await
            },
            "controller.command.review"=>{
                #[derive(Deserialize)] #[serde(rename_all="camelCase",deny_unknown_fields)] struct Review { epoch:String,command_id:String,acknowledge_unknown:bool }
                let p:Review=parse(params)?;
                if !p.acknowledge_unknown {return Err(fail("confirmation_required","review the unknown outcome explicitly"));}
                self.review_command(p.epoch,p.command_id).await
            },
            "controller.status"=>serde_json::to_value(self.status()).map_err(|_|protocol()),
            "controller.profiles"=>Ok(Value::Array(self.profiles.list().await?.into_iter().map(|p|json!({"name":p.name,"profileId":p.profile_id,"origin":p.origin,"hostId":p.host_id,"trust":p.trust})).collect())),
            "controller.connect"=>{let p:Name=parse(params)?;self.connect(&p.profile).await?;Ok(json!({"ok":true}))},
            "controller.disconnect"=>{self.disconnect().await;Ok(json!({"ok":true}))},
            "controller.profile.forget"=>{
                #[derive(Deserialize)] #[serde(rename_all="camelCase",deny_unknown_fields)]
                struct Forget { profile:String, profile_id:String, origin:String, host_id:String, confirm:bool }
                let p:Forget=parse(params)?;
                if !p.confirm {return Err(fail("confirmation_required","confirm local profile and draft removal"));}
                // Serialize with profile selection, pairing and disconnect.
                let _lifecycle=self.lifecycle.lock().await;
                if self.status().profile.as_deref()==Some(p.profile.as_str()) {
                    return Err(fail("profile_in_use","disconnect this profile before forgetting it"));
                }
                self.profiles.forget_reviewed(&p.profile,&p.profile_id,&p.origin,&p.host_id).await?;
                Ok(json!({"ok":true}))
            },
            "controller.probe"=>{#[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Probe{origin:String} let p:Probe=parse(params)?;Ok(json!({"fingerprint":probe_fingerprint(&p.origin).await?,"verified":false}))},
            "controller.pair"=>{
                let mut p:Pair=parse(params)?;
                let private=zeroize::Zeroizing::new(std::mem::take(&mut p.invitation));
                let trust=match p.fingerprint {Some(pin) if p.confirm_pin=>TlsTrust::CertificateSha256{sha256:normalize_fingerprint(&pin)?},Some(_)=>return Err(fail("confirmation_required","verify the host fingerprint before approving it")),None=>TlsTrust::System};
                let mut l=self.lifecycle.lock().await;
                if self.status().phase!="idle" && self.status().phase!="failed" {return Err(fail("invalid_state","disconnect before pairing another host"));}
                self.profiles.ensure_new(&p.profile).await?;self.profiles.check_store().await?;
                let endpoint=RemoteEndpoint::new(&p.origin,trust).await?;
                let (id,secret)=parse_invitation(&private,endpoint.origin())?;
                let pending=endpoint.begin_pairing(&id,&secret,&p.label,ClientKind::Desktop).await?;
                let result=json!({"requestId":pending.request_id,"expiresAt":pending.expires_at});
                let epoch=self.reset(&mut l);
                self.shared.change(&epoch,|v|{v.status.phase="pairing".into();v.status.pairing_request_id=Some(pending.request_id.clone());v.status.profile=Some(p.profile.clone());v.status.origin=Some(endpoint.origin().into());});
                l.pairing=Some(Pairing{profile:p.profile,endpoint,pending});Ok(result)
            },
            "controller.pair.poll"=>{
                let mut l=self.lifecycle.lock().await;
                let mut pairing=l.pairing.take().ok_or_else(||fail("invalid_state","no pairing request is pending"))?;
                let epoch=self.status().epoch;
                match pairing.pending.poll().await {
                    Ok(PairingPoll::Pending)=>{l.pairing=Some(pairing);Ok(json!({"approved":false}))},
                    Ok(PairingPoll::Approved(credentials))=>{
                        let result=self.profiles.install(&pairing.profile,&pairing.endpoint,credentials,ClientKind::Desktop).await;
                        match result {Ok(_)=>{self.shared.phase(&epoch,"idle",None);Ok(json!({"approved":true,"profile":pairing.profile}))},Err(e)=>{self.shared.phase(&epoch,"failed",Some(&e));Err(e)}}
                    },
                    Err(e)=>{self.shared.phase(&epoch,"failed",Some(&e));Err(e)}
                }
            },
            _=>Err(fail("method_not_found","unknown native controller operation"))
        }
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        let v = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
        if let Some(r) = &v.remote {
            r.close();
        }
    }
}
fn parse<T: serde::de::DeserializeOwned>(p: Value) -> Result<T, ClientError> {
    serde_json::from_value(p).map_err(|_| fail("invalid_params", "invalid controller parameters"))
}
async fn apply(
    shared: &Shared,
    epoch: &str,
    remote: &RemoteClient,
    event: &ReliableEvent,
) -> Result<(), ClientError> {
    if event.resource.kind != ResourceKind::Workflow
        || event.resource.scope != "host"
        || event.topic != "workflow.changed"
    {
        return Err(protocol());
    }
    let result = remote
        .call_value_with_meta("workflows.get", json!({"id":event.resource.id}), None)
        .await;
    let (graph, versions) = match result {
        Ok(reply) => (
            Some(reply.result),
            reply.meta.ok_or_else(protocol)?.resource_versions,
        ),
        Err(e) if e.code == "workflow_not_found" => (
            None,
            serde_json::from_value(
                e.data
                    .and_then(|mut v| v.get_mut("resourceVersions").map(Value::take))
                    .ok_or_else(protocol)?,
            )
            .map_err(|_| protocol())?,
        ),
        Err(e) => return Err(e),
    };
    let version = versions
        .into_iter()
        .find(|v| {
            v.kind == ResourceKind::Workflow && v.scope == "host" && v.id == event.resource.id
        })
        .ok_or_else(protocol)?;
    if graph
        .as_ref()
        .is_some_and(|g| g.get("id").and_then(Value::as_str) != Some(version.id.as_str()))
    {
        return Err(protocol());
    }
    let revision =
        tiktools_collaboration::stream_sequence(&version.revision).map_err(|_| protocol())?;
    if revision
        < tiktools_collaboration::stream_sequence(&event.resource.revision)
            .map_err(|_| protocol())?
    {
        return Err(protocol());
    }
    let version = ResourceVersion {
        kind: ResourceKind::Workflow,
        id: version.id,
        scope: version.scope,
        revision: version.revision,
    };
    let mut over = false;
    shared.change(epoch, |v| {
        if v.rows
            .get(&version.id)
            .is_some_and(|r| r.version.revision.parse::<u64>().unwrap_or(0) > revision)
        {
            return;
        }
        match graph {
            Some(graph) => {
                v.rows
                    .insert(version.id.clone(), WorkflowRow { graph, version });
            }
            None => {
                v.rows.remove(&version.id);
            }
        }
        let size = serde_json::to_vec(&v.rows).map_or(usize::MAX, |v| v.len());
        if size > SNAPSHOT_BYTES {
            v.rows.clear();
            over = true;
        }
    });
    if over {
        Err(fail(
            "too_large",
            "controller projection exceeds its byte budget",
        ))
    } else {
        Ok(())
    }
}
async fn run(
    shared: Arc<Shared>,
    commands: Arc<Mutex<()>>,
    epoch: String,
    mut remote: RemoteClient,
    mut recovery: Option<RecoveryStream>,
) {
    let mut backoff = ReconnectBackoff::default();
    let mut clock = tokio::time::interval_at(
        tokio::time::Instant::now() + points::period(&remote),
        points::period(&remote),
    );
    clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        let result = if let Some(stream) = &mut recovery {
            tokio::select! {
                event=stream.next_event()=>match event {
                    Ok(event) if event.resource.kind==ResourceKind::Viewer && event.topic=="points.changed"=>{points::dirty(&shared,&epoch);stream.acknowledge(&event)},
                    Ok(event) if event.resource.kind==ResourceKind::Automation=>match automations::apply(&shared,&epoch,&remote,&event).await {Ok(())=>stream.acknowledge(&event),Err(e)=>Err(e)},
                    Ok(event)=>match apply(&shared,&epoch,&remote,&event).await {Ok(())=>stream.acknowledge(&event),Err(e)=>Err(e)},
                    Err(e)=>Err(e),
                },
                _=clock.tick(), if points::readable(&remote)=>{
                    if points::due(&shared,&epoch) {
                        match points::refresh(&shared,&epoch,&remote).await {
                            Ok(())=>Ok(()),
                            Err(e) if matches!(e.code.as_str(),"too_large"|"internal"|"storage")=>{points::failure(&shared,&epoch,&e);Ok(())},
                            Err(e)=>Err(e),
                        }
                    } else {Ok(())}
                }
            }
        } else {
            // Without a domain stream there is no epoch-gap notification.
            // Refresh hello when the live identity's permissions change.
            tokio::time::sleep(Duration::from_secs(1)).await;
            if remote.is_closed() {
                Err(network())
            } else {
                match remote
                    .call_value_with_meta("collaboration.me", json!({}), None)
                    .await
                {
                    Ok(me)
                        if me.result.get("grantRevision").and_then(Value::as_str)
                            == Some(remote.hello().grant_revision.as_str()) =>
                    {
                        continue
                    }
                    Ok(_) => Err(fail("resync_required", "remote permissions changed")),
                    Err(e) => Err(e),
                }
            }
        };
        if result.is_ok() {
            continue;
        }
        let e = result.unwrap_err();
        if !matches!(
            e.code.as_str(),
            "transport"
                | "forbidden"
                | "resync_required"
                | "unauthorized"
                | "overloaded"
                | "rate_limited"
                | "timeout"
        ) {
            remote.close();
            shared.change(&epoch, |v| {
                v.rows.clear();
                automations::clear(v);
                v.remote = None;
            });
            shared.phase(&epoch, "failed", Some(&e));
            return;
        }
        shared.change(&epoch, points::clear);
        shared.phase(&epoch, "reconnecting", Some(&e));
        loop {
            tokio::time::sleep(backoff.next_delay()).await;
            let mut reconnected = if let Some(stream) = &mut recovery {
                stream.reconnect().await.map(|mode| {
                    (
                        stream.connection().clone(),
                        if mode == RecoveryMode::Snapshot {
                            stream.baseline().cloned()
                        } else {
                            None
                        },
                    )
                })
            } else {
                remote.close();
                remote.reopen().await.map(|r| (r, None))
            };
            // A new grant can remove the old stream's domains. Negotiate the
            // same profile again, then construct only its current domains.
            // No business command is retried in this recovery path.
            let changed_domains = reconnected.as_ref().is_ok_and(|(r, _)| {
                recovery
                    .as_ref()
                    .is_some_and(|s| s.domains() != &points::domains(r))
            });
            let rebuild = changed_domains
                || reconnected
                    .as_ref()
                    .is_err_and(|e| matches!(e.code.as_str(), "forbidden" | "unsupported_remote"));
            if rebuild {
                if let Ok((r, _)) = &reconnected {
                    r.close();
                }
                reconnected = remote.reopen().await.map(|r| (r, None));
            }
            if let Ok((r, baseline)) = &mut reconnected {
                if rebuild || recovery.is_none() {
                    shared.change(&epoch, |v| {
                        v.rows.clear();
                        automations::clear(v);
                    });
                    recovery = None;
                    match points::stream(r).await {
                        Ok(stream) => {
                            *baseline = stream.as_ref().and_then(|s| s.baseline()).cloned();
                            recovery = stream;
                        }
                        Err(e) => reconnected = Err(e),
                    }
                }
            }
            match reconnected {
                Ok((r, baseline)) => {
                    remote = r;
                    shared.change(&epoch, |v| v.remote = Some(remote.clone()));
                    points::reset_scope(&shared, &epoch, &remote);
                    if let Err(e) = points::refresh(&shared, &epoch, &remote).await {
                        if matches!(
                            e.code.as_str(),
                            "forbidden" | "unauthorized" | "resync_required" | "transport"
                        ) {
                            shared.phase(&epoch, "reconnecting", Some(&e));
                            continue;
                        }
                        points::failure(&shared, &epoch, &e);
                    }
                    if let Err(e) = shared.ready(&epoch, &remote, baseline.as_ref()) {
                        remote.close();
                        shared.change(&epoch, |v| {
                            automations::clear(v);
                            v.rows.clear();
                            v.remote = None;
                        });
                        shared.phase(&epoch, "failed", Some(&e));
                        return;
                    }
                    clock = tokio::time::interval_at(
                        tokio::time::Instant::now() + points::period(&remote),
                        points::period(&remote),
                    );
                    clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    commands::schedule_resolve(shared.clone(), commands.clone(), epoch.clone());
                    backoff.reset();
                    break;
                }
                Err(e)
                    if matches!(
                        e.code.as_str(),
                        "transport" | "profile_in_use" | "overloaded" | "rate_limited" | "timeout"
                    ) =>
                {
                    shared.phase(&epoch, "reconnecting", Some(&e));
                }
                Err(e) => {
                    remote.close();
                    shared.change(&epoch, |v| {
                        v.rows.clear();
                        automations::clear(v);
                        v.remote = None;
                    });
                    shared.phase(&epoch, "failed", Some(&e));
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
