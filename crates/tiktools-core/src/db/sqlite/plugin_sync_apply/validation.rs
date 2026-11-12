use super::*;

pub(super) fn valid_intent(intent: &PluginApplyIntent) -> bool {
    if !valid_token(&intent.intent_id)
        || intent.source_generation <= 0
        || intent.origin.is_empty()
        || intent.origin.len() > 2048
        || intent.account_id.is_empty()
        || intent.account_id.len() > 1024
        || intent
            .origin
            .chars()
            .chain(intent.account_id.chars())
            .any(char::is_control)
    {
        return false;
    }
    let mut validator = SyncState::new(intent.origin.clone(), intent.account_id.clone());
    let installed = V1Edit::Put(PluginSelection {
        schema_version: 1,
        plugin_id: intent.plugin_id.clone(),
        version: intent.installed_version.clone(),
        enabled: intent.expected_enabled,
    });
    if validator
        .stage(intent.plugin_id.clone(), installed)
        .is_err()
    {
        return false;
    }
    if let Some(pending) = &intent.reviewed_pending {
        if pending.base_revision.is_some_and(|revision| revision <= 0)
            || validator
                .stage(intent.plugin_id.clone(), pending.edit.clone())
                .is_err()
        {
            return false;
        }
    }
    for record in [&intent.base_record, &intent.remote].into_iter().flatten() {
        if record.collection != "plugin-selections"
            || record.key != intent.plugin_id
            || record.revision <= 0
            || record.deleted != record.value.is_none()
            || record.value.as_ref().is_some_and(|selection| {
                selection.schema_version != 1
                    || selection.plugin_id != intent.plugin_id
                    || validator
                        .stage(intent.plugin_id.clone(), V1Edit::Put(selection.clone()))
                        .is_err()
            })
        {
            return false;
        }
    }
    if let (Some(base), Some(pending)) = (&intent.base_record, &intent.reviewed_pending) {
        if pending.base_revision != Some(base.revision) {
            return false;
        }
    }
    // A previously observed record cannot become an absent export without
    // an explicit reset protocol; treat that as a rewind, not a deletion.
    if intent.base_record.is_some() && intent.remote.is_none() {
        return false;
    }
    if let (Some(base), Some(remote)) = (&intent.base_record, &intent.remote) {
        if remote.revision < base.revision || (remote.revision == base.revision && remote != base) {
            return false;
        }
    }
    match intent.reason {
        PluginApplyReason::Pulled => intent.reviewed_pending.is_none() && intent.remote.is_some(),
        PluginApplyReason::ReviewedConflict => intent
            .reviewed_pending
            .as_ref()
            .is_some_and(|pending| pending.conflict),
        PluginApplyReason::ReviewedUncertain => intent.reviewed_pending.is_some(),
    }
}

pub(super) fn payload(intent: &PluginApplyIntent) -> Result<String, DatabaseError> {
    if !valid_intent(intent) {
        return Err(DatabaseError::Invalid("invalid plugin apply intent".into()));
    }
    let serialized = serde_json::to_string(intent)?;
    if serialized.len() > 16 * 1024 {
        return Err(DatabaseError::Invalid(
            "plugin apply intent too large".into(),
        ));
    }
    Ok(serialized)
}
