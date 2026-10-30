//! Strict declarative action mapping; no console commands or RCON framing.
use l4d2api_protocol as proto;
use serde_json::Value;
use tiktools_plugin_sdk::{PluginError, PluginResult};

pub const TEST: &str = "l4d2.test_connection";
pub const COMMON: &str = "l4d2.spawn_common";
pub const SPECIAL: &str = "l4d2.spawn_infected";
pub const ITEM: &str = "l4d2.spawn_item";
pub const WEAPON: &str = "l4d2.spawn_weapon";

#[derive(Debug)]
pub struct ParsedAction {
    pub action: proto::Action,
    pub label: &'static str,
}

fn count(config: &Value, max: u32) -> PluginResult<u32> {
    let n = config
        .get("count")
        .and_then(Value::as_u64)
        .ok_or_else(|| PluginError::invalid_request("count must be a positive integer"))?;
    if n == 0 || n > u64::from(max) {
        return Err(PluginError::invalid_request(format!(
            "count must be 1..={max}"
        )));
    }
    Ok(n as u32)
}

fn token<'a>(config: &'a Value, key: &str, allowed: &[&str]) -> PluginResult<&'a str> {
    let name = config
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| PluginError::invalid_request(format!("{key} must be a string")))?;
    if !allowed.contains(&name) {
        return Err(PluginError::invalid_request(format!(
            "unsupported {key} value"
        )));
    }
    Ok(name)
}

impl ParsedAction {
    pub fn parse(id: &str, config: &Value) -> PluginResult<Self> {
        if !config.is_object() {
            return Err(PluginError::invalid_request(
                "action config must be an object",
            ));
        }
        let (action, label) = match id {
            TEST => (proto::Action::TestConnection, "Connection"),
            COMMON => (
                proto::Action::SpawnCommon {
                    count: count(config, proto::COMMON_MAX)?,
                },
                "Common infected",
            ),
            SPECIAL => {
                let class = token(config, "class", &proto::INFECTED_ALLOWLIST)?;
                let n = count(
                    config,
                    if class == "tank" {
                        1
                    } else {
                        proto::SPECIAL_MAX
                    },
                )?;
                (
                    proto::Action::SpawnInfected {
                        class: class.to_owned(),
                        count: n,
                    },
                    "Special infected",
                )
            }
            ITEM => {
                let item = token(config, "item", &proto::ITEM_ALLOWLIST)?;
                (
                    proto::Action::SpawnItem {
                        item: item.to_owned(),
                        count: count(config, proto::ITEM_MAX)?,
                    },
                    "Item",
                )
            }
            WEAPON => {
                let weapon = token(config, "weapon", &proto::WEAPON_ALLOWLIST)?;
                (
                    proto::Action::SpawnWeapon {
                        weapon: weapon.to_owned(),
                        count: count(config, proto::WEAPON_MAX)?,
                    },
                    "Weapon",
                )
            }
            _ => return Err(PluginError::unsupported(id)),
        };
        Ok(Self { action, label })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_declared_action_parses() {
        for (id, config) in [
            (TEST, json!({})),
            (ITEM, json!({"item":"first_aid_kit","count":2})),
            (WEAPON, json!({"weapon":"weapon_rifle_ak47","count":1})),
            (COMMON, json!({"count":10})),
            (SPECIAL, json!({"class":"tank","count":1})),
        ] {
            assert!(ParsedAction::parse(id, &config).is_ok(), "{id}");
        }
    }

    #[test]
    fn invalid_inputs_are_rejected_before_ipc() {
        for (id, config) in [
            (ITEM, json!({"item":"molotov;quit","count":1})),
            (WEAPON, json!({"weapon":"weapon_custom","count":1})),
            (COMMON, json!({"count":11})),
            (COMMON, json!({"count":2.5})),
            (SPECIAL, json!({"class":"tank","count":2})),
            (ITEM, json!({"item":"pain_pills","count":0})),
        ] {
            assert!(ParsedAction::parse(id, &config).is_err(), "{id} {config}");
        }
    }
}
