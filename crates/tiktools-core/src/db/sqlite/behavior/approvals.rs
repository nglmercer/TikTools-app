//! Retained Owner approvals. Every admission uses the business writer's own
//! reservation; the collaboration authorization lock spans the actual commit.
use super::*;
use crate::collaboration::{ExecutionAuthorizations, ExecutionIdentity};
use serde::Serialize;
use std::collections::HashSet;

const APPROVAL_BYTES: usize = 4 * 1024 * 1024;
const APPROVALS: usize = 4096;
const DEFINITIONS: usize = 513;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BehaviorApprovalEnableState {
    pub scope: String,
    pub id: String,
    pub enabled: bool,
}

pub(crate) struct BehaviorApprovalActivation<'a> {
    pub table: &'a str,
    pub id: &'a str,
    pub reviewed_versions: &'a [ResourceVersion],
    pub preview_digest: &'a str,
    pub enable_states: &'a [BehaviorApprovalEnableState],
    pub acknowledged: bool,
}

/// Constructed by trusted policy code from the supplied transaction review and
/// current identities. It is never deserialized from an approval request.
pub(crate) struct BehaviorApprovalBinding {
    pub manifest: Value,
    pub bindings: Value,
}

/// Validated storage snapshot, not execution authority. Runtime admission must
/// compare the complete binding with the current catalog and current subjects.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BehaviorApprovalSnapshot {
    pub scope: String,
    pub id: String,
    pub generation: String,
    pub policy_version: u32,
    pub manifest_digest: String,
    pub binding: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Definition {
    scope: String,
    id: String,
    revision: i64,
    provenance_generation: Option<i64>,
    present: bool,
    content_digest: Option<String>,
}

fn invalid() -> DatabaseError {
    DatabaseError::Invalid("automation approval binding is inconsistent".into())
}
fn canonical_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.contains('*')
        && !value.chars().any(char::is_control)
}
fn decimal(value: &str) -> Option<i64> {
    value
        .parse::<i64>()
        .ok()
        .filter(|n| *n >= 0 && n.to_string() == value)
}
fn hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, DatabaseError> {
    value.get(key).and_then(Value::as_str).ok_or_else(invalid)
}
fn table(scope: &str) -> Result<&'static str, DatabaseError> {
    match scope {
        "action" => Ok("behavior_actions"),
        "event" => Ok("behavior_events"),
        _ => Err(invalid()),
    }
}

// Match the review digest's explicit object ordering and bounded recursion.
pub(super) fn canonical(value: &Value, depth: usize) -> Result<Value, DatabaseError> {
    if depth > 64 {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    Ok(match value {
        Value::Object(values) => {
            let sorted: std::collections::BTreeMap<_, _> = values.iter().collect();
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, value)| Ok((key.clone(), canonical(value, depth + 1)?)))
                    .collect::<Result<_, DatabaseError>>()?,
            )
        }
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|v| canonical(v, depth + 1))
                .collect::<Result<_, _>>()?,
        ),
        other => other.clone(),
    })
}
fn digest(domain: &str, value: &Value) -> Result<String, DatabaseError> {
    let encoded = serde_json::to_string(&canonical(value, 0)?)?;
    if encoded.len() > APPROVAL_BYTES {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    Ok(crate::collaboration::hash_secret(&format!(
        "tiktools/automation-manifest/v1/{domain}\0{encoded}"
    )))
}

fn definition(value: &Value) -> Result<Definition, DatabaseError> {
    if !value.as_object().is_some_and(|o| o.len() == 6) {
        return Err(invalid());
    }
    let scope = string(value, "kind")?;
    table(scope)?;
    let id = string(value, "id")?;
    if !canonical_id(id) || id.len() > 128 {
        return Err(invalid());
    }
    let revision = decimal(string(value, "revision")?).ok_or_else(invalid)?;
    let present = value
        .get("exists")
        .and_then(Value::as_bool)
        .ok_or_else(invalid)?;
    let content_digest = match value.get("contentDigest") {
        Some(Value::Null) if !present => None,
        Some(Value::String(v)) if present && hex(v) => Some(v.clone()),
        _ => return Err(invalid()),
    };
    let provenance_generation = match value.get("provenance") {
        Some(Value::Null) if revision == 0 && !present => None,
        Some(p) if revision > 0 && p.as_object().is_some_and(|o| o.len() == 2) => {
            let generation = decimal(string(p, "generation")?)
                .filter(|n| *n > 0)
                .ok_or_else(invalid)?;
            let a = p.get("authority").ok_or_else(invalid)?;
            match string(a, "kind")? {
                "owner" if a.as_object().is_some_and(|o| o.len() == 1) => {}
                "delegated"
                    if a.as_object().is_some_and(|o| o.len() == 3)
                        && canonical_id(string(a, "principalId")?)
                        && canonical_id(string(a, "sessionId")?) => {}
                _ => return Err(invalid()),
            }
            Some(generation)
        }
        _ => return Err(invalid()),
    };
    Ok(Definition {
        scope: scope.into(),
        id: id.into(),
        revision,
        provenance_generation,
        present,
        content_digest,
    })
}

pub(super) fn validate_manifest(
    manifest: &Value,
) -> Result<(u32, String, Vec<Definition>), DatabaseError> {
    if !manifest.as_object().is_some_and(|o| o.len() == 10)
        || string(manifest, "digestAlgorithm")? != "sha256-canonical-json-v1"
        || !canonical_id(string(manifest, "hostId")?)
        || manifest.get("executionAuthorized").and_then(Value::as_bool) != Some(false)
        || !manifest.get("subjects").is_some_and(Value::is_array)
        || !manifest.get("effects").is_some_and(Value::is_array)
        || !manifest.get("reasons").is_some_and(Value::is_array)
    {
        return Err(invalid());
    }
    let policy_version = manifest
        .get("policyVersion")
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .filter(|v| *v > 0)
        .ok_or_else(invalid)?;
    let manifest_digest = string(manifest, "manifestDigest")?;
    if !hex(manifest_digest) {
        return Err(invalid());
    }
    let mut unsigned = manifest.clone();
    unsigned
        .as_object_mut()
        .ok_or_else(invalid)?
        .remove("manifestDigest");
    if digest("manifest", &unsigned)? != manifest_digest {
        return Err(invalid());
    }
    let mut definitions = vec![definition(manifest.get("primary").ok_or_else(invalid)?)?];
    let deps = manifest
        .get("dependencies")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    if deps.len() >= DEFINITIONS {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    definitions.extend(deps.iter().map(definition).collect::<Result<Vec<_>, _>>()?);
    let mut seen = HashSet::new();
    for (position, d) in definitions.iter().enumerate() {
        if !seen.insert((&d.scope, &d.id))
            || (position > 0 && (definitions[0].scope != "event" || d.scope != "action"))
        {
            return Err(invalid());
        }
    }
    if !definitions[0].present {
        return Err(DatabaseError::NotFound);
    }
    Ok((policy_version, manifest_digest.into(), definitions))
}

pub(super) fn validate_review(
    manifest: &Value,
    review: &Versioned<Value>,
) -> Result<Vec<Definition>, DatabaseError> {
    let (_, _, defs) = validate_manifest(manifest)?;
    let dependencies = review
        .value
        .get("dependencies")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    if defs.len() != review.resource_versions.len() || defs.len() != dependencies.len() + 1 {
        return Err(invalid());
    }
    for (position, (d, version)) in defs.iter().zip(&review.resource_versions).enumerate() {
        let item = if position == 0 {
            &review.value
        } else {
            &dependencies[position - 1]
        };
        let record = item.get("record").ok_or_else(invalid)?;
        let provenance = item.get("provenance").ok_or_else(invalid)?;
        let binding = if position == 0 {
            manifest.get("primary").ok_or_else(invalid)?
        } else {
            &manifest["dependencies"][position - 1]
        };
        if version.kind != crate::collaboration::ResourceKind::Automation
            || d.scope != version.scope
            || d.id != version.id
            || d.revision.to_string() != version.revision
            || d.present == record.is_null()
            || binding.get("provenance") != Some(provenance)
            || d.content_digest
                != if record.is_null() {
                    None
                } else {
                    Some(digest("definition", record)?)
                }
        {
            return Err(invalid());
        }
    }
    Ok(defs)
}

pub(super) fn invalidate(
    db: &Connection,
    scope: &str,
    id: &str,
    at: i64,
) -> Result<(), DatabaseError> {
    let exhausted: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM behavior_execution_approvals a WHERE a.state='active' AND a.generation=9223372036854775807 AND EXISTS(SELECT 1 FROM behavior_execution_approval_dependencies d WHERE d.approval_scope=a.scope AND d.approval_id=a.resource_id AND d.definition_scope=?1 AND d.definition_id=?2))",
        params![scope,id], |r| r.get(0))?;
    if exhausted {
        return Err(invalid());
    }
    db.execute(
        "UPDATE behavior_execution_approvals SET state='invalidated',generation=generation+1,updated_at=?3 WHERE state='active' AND EXISTS(SELECT 1 FROM behavior_execution_approval_dependencies d WHERE d.approval_scope=behavior_execution_approvals.scope AND d.approval_id=behavior_execution_approvals.resource_id AND d.definition_scope=?1 AND d.definition_id=?2)",
        params![scope,id,at])?;
    Ok(())
}

fn next_generation(db: &Connection, scope: &str, id: &str) -> Result<i64, DatabaseError> {
    let previous: Option<i64> = db
        .query_row(
            "SELECT generation FROM behavior_execution_approvals WHERE scope=?1 AND resource_id=?2",
            params![scope, id],
            |r| r.get(0),
        )
        .optional()?;
    match previous {
        None => Ok(1),
        Some(previous) if previous > 0 => previous.checked_add(1).ok_or_else(invalid),
        _ => Err(invalid()),
    }
}

fn store(
    db: &Connection,
    scope: &str,
    id: &str,
    command: &super::super::CommandTransaction,
    binding: BehaviorApprovalBinding,
    review: &Versioned<Value>,
) -> Result<BehaviorApprovalSnapshot, DatabaseError> {
    let defs = validate_review(&binding.manifest, review)?;
    let (policy_version, manifest_digest, _) = validate_manifest(&binding.manifest)?;
    let binding = json!({"manifest":binding.manifest,"bindings":binding.bindings});
    let encoded = serde_json::to_string(&canonical(&binding, 0)?)?;
    if encoded.len() > APPROVAL_BYTES {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    if !canonical_id(&command.principal_id) || !canonical_id(&command.session_id) {
        return Err(invalid());
    }
    // Use the loader's aggregate budget before committing a new active row.
    // Otherwise a valid individual approval could make the next atomic runtime
    // refresh reject the entire catalog after its business mutation committed.
    let (count, bytes): (i64, i64) = db.query_row(
        "SELECT count(*),coalesce(sum(length(CAST(binding_json AS BLOB))),0) FROM behavior_execution_approvals WHERE state='active' AND NOT(scope=?1 AND resource_id=?2)",
        params![scope,id], |r| Ok((r.get(0)?,r.get(1)?)))?;
    if count < 0
        || count as u64 >= APPROVALS as u64
        || bytes < 0
        || bytes as u64 > APPROVAL_BYTES.saturating_sub(encoded.len()) as u64
    {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    let generation = next_generation(db, scope, id)?;
    let at = now();
    db.execute("INSERT INTO behavior_execution_approvals(scope,resource_id,generation,state,policy_version,manifest_digest,binding_json,owner_principal_id,owner_session_id,approved_at,updated_at) VALUES (?1,?2,?3,'active',?4,?5,?6,?7,?8,?9,?9) ON CONFLICT(scope,resource_id) DO UPDATE SET generation=excluded.generation,state='active',policy_version=excluded.policy_version,manifest_digest=excluded.manifest_digest,binding_json=excluded.binding_json,owner_principal_id=excluded.owner_principal_id,owner_session_id=excluded.owner_session_id,approved_at=excluded.approved_at,updated_at=excluded.updated_at",
        params![scope,id,generation,policy_version,manifest_digest,encoded,command.principal_id,command.session_id,at])?;
    db.execute("DELETE FROM behavior_execution_approval_dependencies WHERE approval_scope=?1 AND approval_id=?2",params![scope,id])?;
    for (position, d) in defs.iter().enumerate() {
        db.execute("INSERT INTO behavior_execution_approval_dependencies(approval_scope,approval_id,position,definition_scope,definition_id,revision,provenance_generation,present,content_digest) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![scope,id,position as i64,d.scope,d.id,d.revision,d.provenance_generation,bool_int(d.present),d.content_digest])?;
    }
    Ok(BehaviorApprovalSnapshot {
        scope: scope.into(),
        id: id.into(),
        generation: generation.to_string(),
        policy_version,
        manifest_digest,
        binding,
    })
}

fn apply_plan(
    tx: &Transaction<'_>,
    request: &BehaviorApprovalActivation<'_>,
    review: &Versioned<Value>,
) -> Result<Vec<ResourceVersion>, DatabaseError> {
    let primary_scope = versions::scope(request.table)?;
    let mut seen = HashSet::new();
    let mut primary = false;
    let mut changed = Vec::new();
    if request.enable_states.len() > DEFINITIONS {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    for state in request.enable_states {
        table(&state.scope)?;
        if !canonical_id(&state.id)
            || state.id.len() > 128
            || !seen.insert((&state.scope, &state.id))
        {
            return Err(invalid());
        }
        let position = review
            .resource_versions
            .iter()
            .position(|v| v.scope == state.scope && v.id == state.id)
            .ok_or_else(invalid)?;
        if position == 0 {
            if !state.enabled || state.scope != primary_scope || state.id != request.id {
                return Err(invalid());
            }
            primary = true;
            toggle(tx, request.table, request.id, true)?.ok_or(DatabaseError::NotFound)?;
        } else {
            let source = &review.value["dependencies"][position - 1];
            if source["record"].is_null()
                || source["provenance"]["authority"]["kind"].as_str() != Some("delegated")
            {
                return Err(invalid());
            }
            let old_enabled = source["record"]["enabled"].as_bool().ok_or_else(invalid)?;
            if old_enabled != state.enabled {
                let previous = versions::current(tx, "behavior_actions", &state.id)?;
                toggle(tx, "behavior_actions", &state.id, state.enabled)?
                    .ok_or(DatabaseError::NotFound)?;
                changed.push(versions::advance(
                    tx,
                    "behavior_actions",
                    &state.id,
                    previous,
                    false,
                    if state.enabled { "enabled" } else { "disabled" },
                    now(),
                )?);
            }
        }
    }
    if !primary {
        return Err(invalid());
    }
    let previous = versions::current(tx, request.table, request.id)?;
    let primary = versions::advance(
        tx,
        request.table,
        request.id,
        previous,
        false,
        "approved",
        now(),
    )?;
    changed.insert(0, primary);
    Ok(changed)
}

impl DatabaseManager {
    pub(crate) fn execute_behavior_approval_activation(
        &self,
        request: &BehaviorApprovalActivation<'_>,
        command: &super::super::CommandTransaction,
        resolve: impl FnOnce(&Versioned<Value>) -> Result<ExecutionAuthorizations, DatabaseError>,
        mut build: impl FnMut(
            &Versioned<Value>,
            &[ExecutionIdentity],
        ) -> Result<BehaviorApprovalBinding, DatabaseError>,
    ) -> Result<(Versioned<Value>, bool), DatabaseError> {
        if command.authorization.is_some() {
            return Err(DatabaseError::CommandAuthorization(
                crate::collaboration::Error::Forbidden,
            ));
        }
        command.request.validate().map_err(|_| invalid())?;
        let scope = versions::scope(request.table)?;
        let hash = super::super::commands::payload_hash(
            "automation.approval.activate",
            &json!({"kind":scope,"id":request.id,"reviewedVersions":request.reviewed_versions,
                "manifestDigest":request.preview_digest,"enablePlan":request.enable_states,"acknowledgeEffects":request.acknowledged}),
            &command.request.expected_revision,
        )?;
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        super::super::commands::prune(&tx)?;
        if let Some(result) = super::super::commands::replay(&tx, command, &hash)? {
            tx.commit()?;
            return Ok((result, false));
        }
        if !request.acknowledged || !hex(request.preview_digest) {
            return Err(invalid());
        }
        review::check_versions(
            &tx,
            request.table,
            request.id,
            &command.request.expected_revision,
            request.reviewed_versions,
        )?;
        let before = review::snapshot(&tx, request.table, request.id)?;
        let authorizations = resolve(&before)?;
        authorizations.with_current(|identities| {
            let preview = build(&before,identities)?;
            validate_review(&preview.manifest,&before)?;
            if string(&preview.manifest,"manifestDigest")? != request.preview_digest { return Err(invalid()); }
            apply_plan(&tx,request,&before)?;
            let after = review::snapshot(&tx,request.table,request.id)?;
            let final_binding = build(&after,identities)?;
            let approved = store(&tx,scope,request.id,command,final_binding,&after)?;
            let result = Versioned::new(json!({"kind":scope,"id":request.id,"enabled":true,"approved":true,
                "policyVersion":approved.policy_version,"manifestDigest":approved.manifest_digest,"approvalGeneration":approved.generation}),after.resource_versions);
            let result = super::super::commands::finish(&tx,command,&hash,"automation.approval.activate",&command.request.expected_revision,result)?;
            for identity in identities { identity.check_deadline().map_err(DatabaseError::CommandAuthorization)?; }
            // Do not replace this with behavior_transaction: its commit occurs
            // after the callback and would open an authorization race.
            if command.request.command_id.starts_with("owner-approval-") {
                crate::control::owner_approval::owner_approval_crash("before_business_commit");
            }
            tx.commit()?;
            if command.request.command_id.starts_with("owner-approval-") {
                crate::control::owner_approval::owner_approval_crash("after_business_commit");
            }
            Ok((result,true))
        }).map_err(DatabaseError::CommandAuthorization)?
    }

    pub(crate) fn execute_behavior_approval_revocation(
        &self,
        table: &str,
        id: &str,
        acknowledged: bool,
        command: &super::super::CommandTransaction,
    ) -> Result<(Versioned<Value>, bool), DatabaseError> {
        if command.authorization.is_some() {
            return Err(DatabaseError::CommandAuthorization(
                crate::collaboration::Error::Forbidden,
            ));
        }
        command.request.validate().map_err(|_| invalid())?;
        let scope = versions::scope(table)?;
        let hash = super::super::commands::payload_hash(
            "automation.approval.revoke",
            &json!({"kind":scope,"id":id,"acknowledgeRevocation":acknowledged}),
            &command.request.expected_revision,
        )?;
        let mut db = self.open(&self.automation_path())?;
        db.execute_batch("PRAGMA synchronous=FULL;")?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        super::super::commands::prune(&tx)?;
        if let Some(result) = super::super::commands::replay(&tx, command, &hash)? {
            tx.commit()?;
            return Ok((result, false));
        }
        if !acknowledged {
            return Err(invalid());
        }
        let previous = versions::check(&tx, table, id, Some(&command.request.expected_revision))?;
        let generation = next_generation(&tx, scope, id)?;
        let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM behavior_execution_approvals WHERE scope=?1 AND resource_id=?2)",params![scope,id],|r|r.get(0))?;
        if !exists {
            return Err(DatabaseError::NotFound);
        }
        tx.execute("UPDATE behavior_execution_approvals SET generation=?3,state='revoked',updated_at=?4 WHERE scope=?1 AND resource_id=?2",params![scope,id,generation,now()])?;
        // The retained row is already revoked, so the shared invalidator
        // fences reverse references without advancing this identity twice.
        let deleted = versions::row(&tx, table, id)?.is_none();
        let version =
            versions::advance(&tx, table, id, previous, deleted, "approval-revoked", now())?;
        let result = Versioned::new(
            json!({"kind":scope,"id":id,"revoked":true,"approvalGeneration":generation.to_string()}),
            vec![version],
        );
        let result = super::super::commands::finish(
            &tx,
            command,
            &hash,
            "automation.approval.revoke",
            &command.request.expected_revision,
            result,
        )?;
        if command.request.command_id.starts_with("owner-approval-") {
            crate::control::owner_approval::owner_approval_crash("before_business_commit");
        }
        tx.commit()?;
        if command.request.command_id.starts_with("owner-approval-") {
            crate::control::owner_approval::owner_approval_crash("after_business_commit");
        }
        Ok((result, true))
    }
}

fn validate_dependency_rows(
    db: &Connection,
    scope: &str,
    id: &str,
    defs: &[Definition],
) -> Result<(), DatabaseError> {
    if defs.first().is_none_or(|d| d.scope != scope || d.id != id) {
        return Err(invalid());
    }
    let mut deps = db.prepare("SELECT position,definition_scope,definition_id,revision,provenance_generation,present,content_digest FROM behavior_execution_approval_dependencies WHERE approval_scope=?1 AND approval_id=?2 ORDER BY position LIMIT 514")?;
    let mut deps = deps.query(params![scope, id])?;
    for (position, d) in defs.iter().enumerate() {
        let r = deps.next()?.ok_or_else(invalid)?;
        for (column, limit, nullable) in [(1, 6usize, false), (2, 128, false), (6, 64, true)] {
            match r.get_ref(column)? {
                rusqlite::types::ValueRef::Text(bytes) if bytes.len() <= limit => {}
                rusqlite::types::ValueRef::Null if nullable => {}
                _ => return Err(invalid()),
            }
        }
        let present: i64 = r.get(5)?;
        if r.get::<_, i64>(0)? != position as i64
            || r.get::<_, String>(1)? != d.scope
            || r.get::<_, String>(2)? != d.id
            || r.get::<_, i64>(3)? != d.revision
            || r.get::<_, Option<i64>>(4)? != d.provenance_generation
            || present != bool_int(d.present)
            || r.get::<_, Option<String>>(6)? != d.content_digest
        {
            return Err(invalid());
        }
    }
    if deps.next()?.is_some() {
        return Err(invalid());
    }
    Ok(())
}

/// The selected Owner review and startup loader must rely on the same exact,
/// bounded reverse relation before reporting an active approval as current.
pub(super) fn validate_reverse_dependencies(
    db: &Connection,
    scope: &str,
    id: &str,
    manifest: &Value,
) -> Result<(), DatabaseError> {
    let (_, _, defs) = validate_manifest(manifest)?;
    validate_dependency_rows(db, scope, id, &defs)
}

pub(super) fn load(db: &Connection) -> Result<Vec<BehaviorApprovalSnapshot>, DatabaseError> {
    let count: i64 = db.query_row(
        "SELECT count(*) FROM behavior_execution_approvals WHERE state='active'",
        [],
        |r| r.get(0),
    )?;
    if count < 0 || count as u64 > APPROVALS as u64 {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    let mut statement = db.prepare("SELECT scope,resource_id,generation,policy_version,manifest_digest,length(CAST(binding_json AS BLOB)),length(CAST(owner_principal_id AS BLOB)),length(CAST(owner_session_id AS BLOB)) FROM behavior_execution_approvals WHERE state='active' ORDER BY scope,resource_id")?;
    let mut rows = statement.query([])?;
    let mut snapshots = Vec::new();
    let mut bytes = 0usize;
    while let Some(row) = rows.next()? {
        // Borrow SQLite's text first; reject oversized identities before Rust
        // allocates an owned String from potentially damaged private storage.
        for (column, limit) in [(0, 6usize), (1, 128), (4, 64)] {
            match row.get_ref(column)? {
                rusqlite::types::ValueRef::Text(bytes) if bytes.len() <= limit => {}
                _ => return Err(invalid()),
            }
        }
        let scope: String = row.get(0)?;
        let id: String = row.get(1)?;
        let generation: i64 = row.get(2)?;
        let policy_version: u32 = row.get(3)?;
        let manifest_digest: String = row.get(4)?;
        let size: i64 = row.get(5)?;
        let principal_size: i64 = row.get(6)?;
        let session_size: i64 = row.get(7)?;
        table(&scope)?;
        if !canonical_id(&id)
            || id.len() > 128
            || generation <= 0
            || policy_version == 0
            || !hex(&manifest_digest)
            || !(1..=256).contains(&principal_size)
            || !(1..=256).contains(&session_size)
            || size < 0
            || size as u64 > APPROVAL_BYTES.saturating_sub(bytes) as u64
        {
            return Err(invalid());
        }
        bytes += size as usize;
        let (encoded, owner_principal, owner_session): (String,String,String) = db.query_row("SELECT binding_json,owner_principal_id,owner_session_id FROM behavior_execution_approvals WHERE scope=?1 AND resource_id=?2",params![scope,id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        if !canonical_id(&owner_principal) || !canonical_id(&owner_session) {
            return Err(invalid());
        }
        let binding: Value = serde_json::from_str(&encoded)?;
        canonical(&binding, 0)?;
        if !binding
            .as_object()
            .is_some_and(|o| o.len() == 2 && o.contains_key("bindings"))
        {
            return Err(invalid());
        }
        let manifest = binding.get("manifest").ok_or_else(invalid)?;
        let (stored_policy, stored_digest, defs) = validate_manifest(manifest)?;
        if stored_policy != policy_version
            || stored_digest != manifest_digest
            || defs[0].scope != scope
            || defs[0].id != id
        {
            return Err(invalid());
        }
        validate_dependency_rows(db, &scope, &id, &defs)?;
        // A corrupt/missing reverse link cannot silently preserve authority.
        validate_review(manifest, &review::snapshot(db, table(&scope)?, &id)?)?;
        snapshots.push(BehaviorApprovalSnapshot {
            scope,
            id,
            generation: generation.to_string(),
            policy_version,
            manifest_digest,
            binding,
        });
    }
    Ok(snapshots)
}

#[cfg(test)]
mod tests;
