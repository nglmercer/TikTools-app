#![forbid(unsafe_code)]
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]
//! L4D2 Interactive: only local L4D2api file IPC. No RCON/server/installer.
mod command;
mod config;
mod ratelimit;
mod result;
mod session;

use ratelimit::RateLimiter;
use serde_json::Value;
use std::time::Instant;
use tiktools_plugin_sdk::prelude::*;

#[derive(Default)]
struct L4d2Plugin {
    limiter: Option<RateLimiter>,
}

impl L4d2Plugin {
    fn check_rate(&mut self, max_per_minute: u64) -> PluginResult<()> {
        let limiter = self
            .limiter
            .get_or_insert_with(|| RateLimiter::new(max_per_minute));
        if limiter.max_per_minute() != max_per_minute {
            *limiter = RateLimiter::new(max_per_minute);
        }
        limiter.check(Instant::now()).map_err(|delay| {
            PluginError::other(format!(
                "L4D2 action rate limit reached; retry in {}s",
                delay.as_secs().max(1)
            ))
        })
    }

    fn run_action(&mut self, action_type: &str, values: &Value) -> PluginResult<ActionResult> {
        // Validate before any filesystem access or game mutation.
        let parsed = command::ParsedAction::parse(action_type, values)?;
        let settings = config::Settings::load()?;
        if action_type != command::TEST {
            self.check_rate(settings.max_per_minute)?;
        }
        session::execute(&settings, parsed)
    }
}

impl Plugin for L4d2Plugin {
    fn action(&mut self, _context: &PluginContext, call: ActionCall) -> PluginResult<ActionResult> {
        self.run_action(
            call.action_type().unwrap_or_default(),
            &Value::Object(call.config().clone()),
        )
    }
}

tiktools_process_plugin!(L4d2Plugin);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unsupported_action_fails_without_settings() {
        assert!(matches!(
            L4d2Plugin::default().run_action("l4d2.invalid", &json!({})),
            Err(PluginError::UnsupportedAction(_))
        ));
    }

    #[test]
    fn invalid_count_fails_without_game() {
        assert!(matches!(
            L4d2Plugin::default().run_action(command::COMMON, &json!({"count":25})),
            Err(PluginError::InvalidRequest(_))
        ));
    }

    #[test]
    fn rate_limit_enforces_bound() {
        let mut plugin = L4d2Plugin::default();
        assert!(plugin.check_rate(1).is_ok());
        assert!(plugin.check_rate(1).is_err());
    }
}
