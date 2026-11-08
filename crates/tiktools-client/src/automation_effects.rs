//! Strict Owner-only external-effect receipt inspection. Historical receipts
//! never authorize dispatch and this SDK deliberately exposes no retry method.
use crate::{CallResult, ClientError, TikToolsClient};
use serde_json::Value;
pub use tiktools_control_api::modules::automation_effects::{
    EffectGetParams, EffectHistory, EffectHistoryParams, EffectKind, EffectReceipt, EffectState,
};

const GET: &str = "automation.effects.get";
const HISTORY: &str = "automation.effects.history";
pub(crate) const METHODS: &[&str] = &[GET, HISTORY];

fn protocol() -> ClientError {
    ClientError::new("protocol", "invalid external effect receipt")
}
fn digest(value: &Value) -> Result<&str, ClientError> {
    value
        .as_str()
        .filter(|value| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        .ok_or_else(protocol)
}
fn exact<'a>(
    value: &'a Value,
    keys: &[&str],
) -> Result<&'a serde_json::Map<String, Value>, ClientError> {
    let object = value.as_object().ok_or_else(protocol)?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(protocol());
    }
    Ok(object)
}
fn envelope(response: &CallResult<Value>) -> Result<(), ClientError> {
    let metadata = response.meta.as_ref().ok_or_else(protocol)?;
    if !metadata.resource_versions.is_empty()
        || metadata.command.is_some()
        || metadata.host_instance_id.is_some()
        || serde_json::to_vec(&response.result)
            .map_err(|_| protocol())?
            .len()
            > 128 * 1024
    {
        return Err(protocol());
    }
    Ok(())
}
fn receipt(value: &Value) -> Result<EffectReceipt, ClientError> {
    let row = exact(
        value,
        &[
            "effectId",
            "kind",
            "state",
            "acceptedAt",
            "updatedAt",
            "outcomeExpired",
        ],
    )?;
    digest(&row["effectId"])?;
    let accepted = row["acceptedAt"]
        .as_u64()
        .filter(|time| *time <= 9_007_199_254_740_991)
        .ok_or_else(protocol)?;
    row["updatedAt"]
        .as_u64()
        .filter(|time| *time >= accepted && *time <= 9_007_199_254_740_991)
        .ok_or_else(protocol)?;
    if !matches!(row["kind"].as_str(), Some("http" | "audio" | "plugin"))
        || !matches!(
            row["state"].as_str(),
            Some("accepted" | "dispatched" | "succeeded" | "failed" | "unknown")
        )
        || row["outcomeExpired"].as_bool().is_none()
        || (row["outcomeExpired"] == true && row["state"] != "unknown")
    {
        return Err(protocol());
    }
    serde_json::from_value(value.clone()).map_err(|_| protocol())
}
fn get(
    response: &CallResult<Value>,
    requested: &str,
) -> Result<Option<EffectReceipt>, ClientError> {
    envelope(response)?;
    if response.result.is_null() {
        return Ok(None);
    }
    let receipt = receipt(&response.result)?;
    if receipt.effect_id != requested {
        return Err(protocol());
    }
    Ok(Some(receipt))
}
fn history(
    response: &CallResult<Value>,
    params: &EffectHistoryParams,
) -> Result<EffectHistory, ClientError> {
    envelope(response)?;
    let page = exact(&response.result, &["items", "nextBefore"])?;
    let rows = page["items"]
        .as_array()
        .filter(|rows| rows.len() <= params.limit as usize)
        .ok_or_else(protocol)?;
    let mut previous = params.before.as_deref();
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let parsed = receipt(row)?;
        let id = digest(&row["effectId"])?;
        if previous.is_some_and(|previous| id >= previous) {
            return Err(protocol());
        }
        previous = Some(id);
        items.push(parsed);
    }
    let next_before = if page["nextBefore"].is_null() {
        None
    } else {
        let next = digest(&page["nextBefore"])?;
        if items.len() != params.limit as usize
            || Some(next) != items.last().map(|row| row.effect_id.as_str())
        {
            return Err(protocol());
        }
        Some(next.to_owned())
    };
    Ok(EffectHistory { items, next_before })
}

impl TikToolsClient {
    pub async fn automation_effects_get(
        &self,
        params: EffectGetParams,
    ) -> Result<Option<EffectReceipt>, ClientError> {
        params
            .validate()
            .map_err(|error| ClientError::new(error.code, error.message))?;
        let response = self.call_value_with_meta(GET, &params, None).await?;
        get(&response, &params.effect_id)
    }
    pub async fn automation_effects_history(
        &self,
        params: EffectHistoryParams,
    ) -> Result<EffectHistory, ClientError> {
        params
            .validate()
            .map_err(|error| ClientError::new(error.code, error.message))?;
        let response = self.call_value_with_meta(HISTORY, &params, None).await?;
        history(&response, &params)
    }
}

#[cfg(test)]
mod tests;
