//! Private proposal copies and fixed authoring operations, never summary edits.
use super::*;
use profiles::commands::{Intent, Operation, Outcome};
use profiles::proposals::{ProposalDraft, ProposalMode};
use tiktools_collaboration::AutomationFamily;

macro_rules! request {
    ($name:ident {$($extra:tt)*}) => {
        #[derive(Deserialize)]
        #[serde(rename_all="camelCase",deny_unknown_fields)]
        pub(super) struct $name {
            pub epoch:String,pub connection_id:String,pub host_instance_id:String,pub grant_revision:String,
            $($extra)*
        }
        impl $name {fn identity(&self)->Identity<'_>{Identity{epoch:&self.epoch,connection:&self.connection_id,instance:&self.host_instance_id,grant:&self.grant_revision}}}
    }
}
request!(Read {#[serde(default)] pub offset:usize,pub kind:Option<AutomationFamily>,pub id:Option<String>,});
request!(Save {pub kind:AutomationFamily,pub id:String,pub expected_store_revision:String,pub mode:ProposalMode,pub base_revision:String,pub text:String,pub acknowledge_storage:bool,});
request!(Delete {pub kind:AutomationFamily,pub id:String,pub expected_store_revision:String,});
request!(Inspect {pub kind:AutomationFamily,pub id:String,});
request!(Submit {pub kind:AutomationFamily,pub id:String,pub inspection_id:String,pub expected_store_revision:String,pub acknowledge_proposal:bool,#[serde(default)] pub active:bool,});
#[derive(Clone)]
pub(super) struct Inspection {
    token: String,
    epoch: String,
    connection: String,
    instance: String,
    grant: String,
    kind: AutomationFamily,
    id: String,
    revision: String,
    summary: Option<Value>,
}
#[derive(Clone, Copy)]
struct Identity<'a> {
    epoch: &'a str,
    connection: &'a str,
    instance: &'a str,
    grant: &'a str,
}
struct Selection {
    journal: Arc<Journal>,
    remote: RemoteClient,
}
fn canonical(id: &str) -> bool {
    profiles::drafts::valid_id(id)
}
fn permitted(remote: &RemoteClient, kind: AutomationFamily, id: &str) -> bool {
    let h = remote.hello();
    h.capabilities.contains(&Capability::AutomationRead)
        && !h.resource_restrictions.contains_key(&ResourceKind::Live)
        && h.resource_restrictions
            .get(&ResourceKind::Automation)
            .is_none_or(|ids| ids.contains(&kind.grant_id(id)))
}
fn selected_view(
    v: &View,
    p: Identity<'_>,
    target: Option<(AutomationFamily, &str)>,
    write: bool,
) -> Result<Selection, ClientError> {
    let r = v
        .remote
        .as_ref()
        .filter(|r| v.status.phase == "ready" && !r.is_closed() && !r.requires_resync())
        .ok_or_else(|| {
            fail(
                "not_ready",
                "recover the controller before accessing proposals",
            )
        })?;
    let h = r.hello();
    if v.status.epoch != p.epoch
        || h.connection_id != p.connection
        || h.host_instance_id != p.instance
        || h.grant_revision != p.grant
    {
        return Err(fail("stale_view", "proposal connection or grant changed"));
    }
    if !h.capabilities.contains(&Capability::AutomationRead)
        || h.resource_restrictions.contains_key(&ResourceKind::Live)
        || target.is_some_and(|(kind, id)| !canonical(id) || !permitted(r, kind, id))
        || write
            && (!h.capabilities.contains(&Capability::AutomationEdit)
                || h.features.get("automationDrafts") != Some(&true))
    {
        return Err(fail("forbidden", "proposal is unavailable for this access"));
    }
    let j = v
        .journal
        .clone()
        .ok_or_else(|| fail("not_ready", "select a stored profile first"))?;
    let b = j.view().binding;
    if b.host_id != h.host_id
        || b.principal_id != h.principal_id
        || b.session_id != h.session_id
        || Some(b.origin.as_str()) != v.status.origin.as_deref()
    {
        return Err(fail("stale_view", "proposal profile changed"));
    }
    Ok(Selection {
        journal: j,
        remote: r.clone(),
    })
}
fn selected(
    shared: &Shared,
    p: Identity<'_>,
    target: Option<(AutomationFamily, &str)>,
    write: bool,
) -> Result<Selection, ClientError> {
    selected_view(
        &shared.view.read().unwrap_or_else(|e| e.into_inner()),
        p,
        target,
        write,
    )
}
fn reply(p: Identity<'_>, revision: &str) -> Value {
    json!({"epoch":p.epoch,"connectionId":p.connection,"hostInstanceId":p.instance,"grantRevision":p.grant,"storeRevision":revision})
}
fn revision(v: &str) -> bool {
    v.parse::<i64>()
        .ok()
        .is_some_and(|n| n >= 0 && n.to_string() == v)
}
fn version(
    values: &Value,
    kind: AutomationFamily,
    id: &str,
    present: bool,
) -> Result<String, ClientError> {
    let rows = values
        .as_array()
        .filter(|r| r.len() == 1)
        .ok_or_else(protocol)?;
    let v = &rows[0];
    let value = v
        .get("revision")
        .and_then(Value::as_str)
        .filter(|v| revision(v) && (!present || *v != "0"))
        .ok_or_else(protocol)?;
    if v.get("kind").and_then(Value::as_str) != Some("automation")
        || v.get("scope").and_then(Value::as_str) != Some(kind.scope())
        || v.get("id").and_then(Value::as_str) != Some(id)
    {
        return Err(protocol());
    }
    Ok(value.into())
}
fn summary(value: &Value, kind: AutomationFamily, id: &str) -> bool {
    value.as_object().is_some_and(|o| o.len() == 5)
        && value.get("kind").and_then(Value::as_str) == Some(kind.scope())
        && value.get("id").and_then(Value::as_str) == Some(id)
        && value.get("definitionRedacted") == Some(&Value::Bool(true))
        && value.get("enabled").is_some_and(Value::is_boolean)
        && value
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|s| s.chars().count() <= 160)
}
fn captured(shared: &Shared, p: &Submit) -> Result<Selection, ClientError> {
    let v = shared.view.read().unwrap_or_else(|e| e.into_inner());
    let s = selected_view(&v, p.identity(), Some((p.kind, &p.id)), true)?;
    let i = v
        .proposal_inspection
        .as_ref()
        .ok_or_else(|| fail("stale_view", "inspect the target before sending a proposal"))?;
    if i.token != p.inspection_id
        || i.epoch != p.epoch
        || i.connection != p.connection_id
        || i.instance != p.host_instance_id
        || i.grant != p.grant_revision
        || i.kind != p.kind
        || i.id != p.id
    {
        return Err(fail("stale_view", "proposal inspection changed"));
    }
    Ok(s)
}
impl Controller {
    pub(super) async fn proposal_read(&self, p: Read, list: bool) -> Result<Value, ClientError> {
        let target = match (p.kind, p.id.as_deref()) {
            (Some(k), Some(id)) => Some((k, id)),
            (None, None) if list => None,
            _ => return Err(fail("invalid_params", "invalid proposal selection")),
        };
        if list && target.is_some() || !list && p.offset != 0 {
            return Err(fail("invalid_params", "invalid proposal page"));
        }
        let s = selected(&self.shared, p.identity(), target, false)?;
        let (rev, rows) = tokio::task::spawn_blocking(move || s.journal.proposals())
            .await
            .map_err(|_| profiles::proposals::invalid())??;
        let s = selected(&self.shared, p.identity(), target, false)?;
        let mut result = reply(p.identity(), &rev);
        if let Some((kind, id)) = target {
            result["draft"] =
                serde_json::to_value(rows.into_iter().find(|d| d.kind == kind && d.id == id))
                    .map_err(|_| protocol())?;
            return Ok(result);
        }
        let rows:Vec<_>=rows.into_iter().filter(|d|permitted(&s.remote,d.kind,&d.id)).map(|d|json!({"kind":d.kind,"id":d.id,"mode":d.mode,"baseRevision":d.base_revision,"updatedAt":d.updated_at,"textBytes":d.text.len()})).collect();
        if p.offset > rows.len() {
            return Err(fail("invalid_params", "invalid proposal page"));
        }
        let end = rows.len().min(p.offset.saturating_add(32));
        result["rows"] = json!(rows[p.offset..end]);
        result["next"] = if end < rows.len() {
            json!(end)
        } else {
            Value::Null
        };
        Ok(result)
    }
    pub(super) async fn proposal_save(&self, p: Save) -> Result<Value, ClientError> {
        if !p.acknowledge_storage {
            return Err(fail(
                "confirmation_required",
                "confirm retaining the complete proposal, including sensitive values, on this PC",
            ));
        }
        let s = selected(&self.shared, p.identity(), Some((p.kind, &p.id)), false)?;
        let draft = ProposalDraft {
            kind: p.kind,
            id: p.id.clone(),
            mode: p.mode,
            base_revision: p.base_revision.clone(),
            text: p.text.clone(),
            updated_at: profiles::commands::timestamp(),
        };
        let expected = p.expected_store_revision.clone();
        let id = p.id.clone();
        let kind = p.kind;
        let rev = tokio::task::spawn_blocking(move || {
            s.journal.change_proposal(&expected, kind, &id, Some(draft))
        })
        .await
        .map_err(|_| profiles::proposals::invalid())??;
        selected(&self.shared, p.identity(), Some((p.kind, &p.id)), false)?;
        Ok(reply(p.identity(), &rev))
    }
    pub(super) async fn proposal_delete(&self, p: Delete) -> Result<Value, ClientError> {
        let s = selected(&self.shared, p.identity(), Some((p.kind, &p.id)), false)?;
        let expected = p.expected_store_revision.clone();
        let id = p.id.clone();
        let kind = p.kind;
        let rev = tokio::task::spawn_blocking(move || {
            s.journal.change_proposal(&expected, kind, &id, None)
        })
        .await
        .map_err(|_| profiles::proposals::invalid())??;
        selected(&self.shared, p.identity(), Some((p.kind, &p.id)), false)?;
        Ok(reply(p.identity(), &rev))
    }
    pub(super) async fn proposal_source(&self, p: Inspect) -> Result<Value, ClientError> {
        let selection = selected(&self.shared, p.identity(), Some((p.kind, &p.id)), false)?;
        if selection
            .remote
            .hello()
            .features
            .get("automationSharedSource")
            != Some(&true)
            || !selection
                .remote
                .hello()
                .capabilities
                .contains(&Capability::AutomationEdit)
        {
            return Err(fail("forbidden", "shared source is unavailable"));
        }
        let reply = selection
            .remote
            .call_value_with_meta(
                "automation.source.get",
                json!({"kind":p.kind,"id":p.id}),
                None,
            )
            .await?;
        let meta = reply.meta.ok_or_else(protocol)?;
        if meta.command.is_some() {
            return Err(protocol());
        }
        let rev = version(
            &serde_json::to_value(meta.resource_versions).map_err(|_| protocol())?,
            p.kind,
            &p.id,
            true,
        )?;
        let body = &reply.result;
        if body.as_object().is_none_or(|o| {
            o.len() != 4
                || !o
                    .keys()
                    .all(|key| matches!(key.as_str(), "kind" | "id" | "revision" | "record"))
        }) || body.get("kind").and_then(Value::as_str) != Some(p.kind.scope())
            || body.get("id").and_then(Value::as_str) != Some(p.id.as_str())
            || body.get("revision").and_then(Value::as_str) != Some(rev.as_str())
            || body.pointer("/record/id").and_then(Value::as_str) != Some(p.id.as_str())
            || serde_json::to_vec(body).map_err(|_| protocol())?.len() > 256 * 1024
        {
            return Err(protocol());
        }
        let latest = selected(&self.shared, p.identity(), Some((p.kind, &p.id)), false)?;
        if latest.remote.hello().features.get("automationSharedSource") != Some(&true)
            || !latest
                .remote
                .hello()
                .capabilities
                .contains(&Capability::AutomationEdit)
        {
            return Err(fail("forbidden", "shared source is unavailable"));
        }
        Ok(
            json!({"epoch":p.epoch,"connectionId":p.connection_id,"hostInstanceId":p.host_instance_id,"grantRevision":p.grant_revision,"kind":p.kind,"id":p.id,"revision":rev,"record":body["record"]}),
        )
    }
    pub(super) async fn proposal_inspect(&self, p: Inspect) -> Result<Value, ClientError> {
        let s = selected(&self.shared, p.identity(), Some((p.kind, &p.id)), false)?;
        let response = s
            .remote
            .call_value_with_meta("automation.get", json!({"kind":p.kind,"id":p.id}), None)
            .await;
        let (body, rev) = match response {
            Ok(value) => {
                if !summary(&value.result, p.kind, &p.id) {
                    return Err(protocol());
                }
                let meta = value.meta.ok_or_else(protocol)?;
                if meta.command.is_some() {
                    return Err(protocol());
                }
                let rev = version(
                    &serde_json::to_value(meta.resource_versions).map_err(|_| protocol())?,
                    p.kind,
                    &p.id,
                    true,
                )?;
                (Some(value.result), rev)
            }
            Err(error) if error.code == "automation_not_found" => {
                let versions = error
                    .data
                    .as_ref()
                    .and_then(|d| d.get("resourceVersions"))
                    .ok_or_else(protocol)?;
                (None, version(versions, p.kind, &p.id, false)?)
            }
            Err(error) => return Err(error),
        };
        let mut v = self.shared.view.write().unwrap_or_else(|e| e.into_inner());
        selected_view(&v, p.identity(), Some((p.kind, &p.id)), false)?;
        let token = uuid::Uuid::new_v4().to_string();
        v.proposal_inspection = Some(Inspection {
            token: token.clone(),
            epoch: p.epoch.clone(),
            connection: p.connection_id.clone(),
            instance: p.host_instance_id.clone(),
            grant: p.grant_revision.clone(),
            kind: p.kind,
            id: p.id.clone(),
            revision: rev.clone(),
            summary: body.clone(),
        });
        Ok(
            json!({"epoch":p.epoch,"connectionId":p.connection_id,"hostInstanceId":p.host_instance_id,"grantRevision":p.grant_revision,"inspectionId":token,"kind":p.kind,"id":p.id,"revision":rev,"summary":body}),
        )
    }
}
pub(super) async fn mutate(shared: &Shared, p: Submit) -> Result<(), ClientError> {
    if !p.acknowledge_proposal {
        return Err(fail(
            "confirmation_required",
            "confirm sending this saved disabled proposal to the host",
        ));
    }
    let s = captured(shared, &p)?;
    let j = s.journal.clone();
    let (store, rows) = tokio::task::spawn_blocking(move || j.proposals())
        .await
        .map_err(|_| profiles::proposals::invalid())??;
    captured(shared, &p)?;
    if store != p.expected_store_revision {
        return Err(fail(
            "proposal_conflict",
            "saved proposal changed; review its exact saved copy",
        ));
    }
    let d = rows
        .into_iter()
        .find(|d| d.kind == p.kind && d.id == p.id)
        .ok_or_else(profiles::proposals::invalid)?;
    {
        let v = shared.view.read().unwrap_or_else(|e| e.into_inner());
        let i = v.proposal_inspection.as_ref().ok_or_else(protocol)?;
        if d.base_revision != i.revision || (d.mode == ProposalMode::Update) != i.summary.is_some()
        {
            return Err(fail(
                "stale_view",
                "review the target status and explicitly save the intended replacement base",
            ));
        }
    }
    let mut record: Value = serde_json::from_str(&d.text)
        .map_err(|_| fail("invalid_params", "proposal must be a JSON object"))?;
    if !record.is_object()
        || record.get("id").and_then(Value::as_str) != Some(p.id.as_str())
        || record.get("enabled") != Some(&Value::Bool(false))
        || record
            .get(if p.kind == AutomationFamily::Event {
                "trigger"
            } else {
                "typeId"
            })
            .and_then(Value::as_str)
            .is_none_or(|s| s.trim().is_empty())
    {
        return Err(fail(
            "invalid_params",
            "proposal must match its saved ID and family and be explicitly disabled",
        ));
    }
    if p.active {
        let hello = s.remote.hello();
        if hello.features.get("automationReusableWrite") != Some(&true)
            || !hello.capabilities.contains(&Capability::AutomationToggle)
        {
            return Err(fail(
                "forbidden",
                "active automation authoring is unavailable",
            ));
        }
        record["enabled"] = Value::Bool(true);
    }
    let operation = match (d.mode, p.kind, p.active) {
        (ProposalMode::Create, AutomationFamily::Event, false) => Operation::CreateEventDraft,
        (ProposalMode::Update, AutomationFamily::Event, false) => Operation::UpdateEventDraft,
        (ProposalMode::Create, AutomationFamily::Action, false) => Operation::CreateActionDraft,
        (ProposalMode::Update, AutomationFamily::Action, false) => Operation::UpdateActionDraft,
        (ProposalMode::Create, AutomationFamily::Event, true) => Operation::CreateEventActive,
        (ProposalMode::Update, AutomationFamily::Event, true) => Operation::UpdateEventActive,
        (ProposalMode::Create, AutomationFamily::Action, true) => Operation::CreateActionActive,
        (ProposalMode::Update, AutomationFamily::Action, true) => Operation::UpdateActionActive,
    };
    let params = if d.mode == ProposalMode::Create {
        json!({"kind":p.kind,"record":record})
    } else {
        json!({"kind":p.kind,"id":p.id,"record":record})
    };
    if serde_json::to_vec(&params).map_err(|_| protocol())?.len()
        > s.remote.hello().limits.params_bytes
    {
        return Err(fail(
            "invalid_params",
            "proposal exceeds the host parameter budget",
        ));
    }
    let intent = Intent {
        command_id: uuid::Uuid::new_v4().to_string(),
        operation,
        resource_id: p.id.clone(),
        host_instance_id: p.host_instance_id.clone(),
        expected_revision: Some(d.base_revision),
        delta: None,
        currency_name: None,
        proposal_revision: Some(store.clone()),
        created_at: profiles::commands::timestamp(),
    };
    let j = s.journal.clone();
    let pending = intent.clone();
    tokio::task::spawn_blocking(move || j.begin(pending))
        .await
        .map_err(|_| fail("command_journal_unavailable", "journal task failed"))??;
    if let Err(error) = captured(shared, &p) {
        commands::finish(
            &s.journal,
            intent,
            Outcome::Rejected,
            None,
            Some(error.code.clone()),
        )
        .await?;
        return Err(error);
    }
    let j = s.journal.clone();
    let latest = tokio::task::spawn_blocking(move || j.proposals())
        .await
        .map_err(|_| profiles::proposals::invalid())??;
    if latest.0 != store {
        let error = fail(
            "proposal_conflict",
            "saved proposal changed before dispatch",
        );
        commands::finish(
            &s.journal,
            intent,
            Outcome::Rejected,
            None,
            Some(error.code.clone()),
        )
        .await?;
        return Err(error);
    }
    captured(shared, &p)?;
    let result = s
        .remote
        .call_value_with_meta(
            operation.method(),
            params,
            Some(RequestMetadata {
                command_id: Some(intent.command_id.clone()),
                host_instance_id: Some(intent.host_instance_id.clone()),
                expected_revision: intent.expected_revision.clone(),
            }),
        )
        .await;
    commands::confirm_reply(&s.journal, intent, result).await
}
