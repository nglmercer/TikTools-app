//! Validation for untrusted plugin UI descriptors.
//!
//! Mirrors the TypeScript normalizer (`src/plugin-ui/normalize.ts`):
//! depth/children/option caps, string length caps, binding charset rules,
//! and the closed node/action allowlists. Anything outside the contract
//! fails closed with [`UiValidationError`].

use serde_json::Value;
use thiserror::Error;

use super::nodes::{LocalizedText, PluginUiNode, PluginUiNodeType};

/// Maximum nesting depth for container nodes.
pub const MAX_UI_DEPTH: usize = 8;
/// Maximum children per container node.
pub const MAX_UI_CHILDREN: usize = 32;
/// Maximum static options on a select node.
pub const MAX_UI_OPTIONS: usize = 256;
/// Maximum length for localized defaults and option sources.
pub const MAX_UI_TEXT: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UiValidationError {
    #[error("invalid ui page id")]
    InvalidPageId,
    #[error("invalid ui page title")]
    InvalidPageTitle,
    #[error("ui tree exceeds max depth")]
    TooDeep,
    #[error("container exceeds max children")]
    TooManyChildren,
    #[error("missing required field `{0}` for node `{1:?}`")]
    MissingField(&'static str, PluginUiNodeType),
    #[error("invalid binding `{0}`")]
    InvalidBinding(String),
    #[error("invalid range bounds")]
    InvalidRange,
    #[error("invalid action for node `{0:?}`")]
    InvalidAction(PluginUiNodeType),
    #[error("invalid localized text")]
    InvalidLocalizedText,
    #[error("invalid option source")]
    InvalidOptionSource,
    #[error("too many static options")]
    TooManyOptions,
    #[error("invalid node key")]
    InvalidKey,
    #[error("invalid variant")]
    InvalidVariant,
    #[error("invalid tone")]
    InvalidTone,
}

/// True for `[A-Za-z][A-Za-z0-9_-]{0,63}` segments.
fn is_key_segment(segment: &str) -> bool {
    if segment.is_empty() || segment.len() > 64 {
        return false;
    }
    let mut chars = segment.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Validates `settings.*` / `local.*` / `source.*` bindings. No expressions.
fn is_valid_binding(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 144 {
        return false;
    }
    let Some((scope, path)) = trimmed.split_once('.') else {
        return false;
    };
    match scope {
        "settings" => {
            if path.is_empty() || path.len() > 128 {
                return false;
            }
            let segments: Vec<&str> = path.split('.').collect();
            if segments.is_empty() || segments.len() > 8 {
                return false;
            }
            if segments
                .iter()
                .any(|s| *s == "__proto__" || *s == "prototype")
            {
                return false;
            }
            segments.iter().all(|s| is_key_segment(s))
        }
        "local" | "source" => !path.is_empty() && !path.contains('.') && is_key_segment(path),
        _ => false,
    }
}

pub(crate) fn validate_localized_text(value: &LocalizedText) -> Result<(), UiValidationError> {
    validate_localized(value)
}

fn validate_localized(value: &LocalizedText) -> Result<(), UiValidationError> {
    if value.default.trim().is_empty() || value.default.len() > MAX_UI_TEXT {
        return Err(UiValidationError::InvalidLocalizedText);
    }
    if value.i18key.len() > 256 {
        return Err(UiValidationError::InvalidLocalizedText);
    }
    Ok(())
}

fn validate_optional_localized(value: &Option<LocalizedText>) -> Result<(), UiValidationError> {
    if let Some(text) = value {
        validate_localized(text)?;
    }
    Ok(())
}

fn validate_key(key: &Option<String>) -> Result<(), UiValidationError> {
    if let Some(key) = key {
        if !is_key_segment(key) {
            return Err(UiValidationError::InvalidKey);
        }
    }
    Ok(())
}

fn validate_option_source(source: &str) -> Result<(), UiValidationError> {
    if source.trim().is_empty() || source.len() > 256 {
        return Err(UiValidationError::InvalidOptionSource);
    }
    Ok(())
}

/// Validates one node recursively (depth-limited).
pub fn validate_ui_node(node: &PluginUiNode, depth: usize) -> Result<(), UiValidationError> {
    if depth > MAX_UI_DEPTH {
        return Err(UiValidationError::TooDeep);
    }
    validate_key(&node.key)?;
    validate_optional_localized(&node.label)?;
    validate_optional_localized(&node.title)?;
    match node.node_type {
        PluginUiNodeType::Stack | PluginUiNodeType::Card => {
            let children = node
                .children
                .as_ref()
                .ok_or(UiValidationError::MissingField("children", node.node_type))?;
            if children.is_empty() || children.len() > MAX_UI_CHILDREN {
                return Err(UiValidationError::TooManyChildren);
            }
            for child in children {
                validate_ui_node(child, depth + 1)?;
            }
            Ok(())
        }
        PluginUiNodeType::Text => {
            let text = node
                .text
                .as_ref()
                .ok_or(UiValidationError::MissingField("text", node.node_type))?;
            validate_localized(text)
        }
        PluginUiNodeType::Status => {
            let text = node
                .text
                .as_ref()
                .ok_or(UiValidationError::MissingField("text", node.node_type))?;
            validate_localized(text)?;
            if let Some(tone) = &node.tone {
                if tone != "info" && tone != "ok" && tone != "error" {
                    return Err(UiValidationError::InvalidTone);
                }
            }
            Ok(())
        }
        PluginUiNodeType::Form => {
            if let Some(schema) = &node.schema {
                if !schema.is_object() {
                    return Err(UiValidationError::MissingField("schema", node.node_type));
                }
            }
            if let Some(hints) = &node.ui_hints {
                if !hints.is_object() {
                    return Err(UiValidationError::MissingField("uiHints", node.node_type));
                }
            }
            Ok(())
        }
        PluginUiNodeType::Select => {
            let bind = node
                .bind
                .as_deref()
                .ok_or(UiValidationError::MissingField("bind", node.node_type))?;
            if !is_valid_binding(bind) {
                return Err(UiValidationError::InvalidBinding(bind.to_string()));
            }
            if let Some(options) = &node.options {
                if options.len() > MAX_UI_OPTIONS {
                    return Err(UiValidationError::TooManyOptions);
                }
                for option in options {
                    if option.value.len() > 256 {
                        return Err(UiValidationError::InvalidOptionSource);
                    }
                    validate_localized(&option.label)?;
                }
            }
            if let Some(source) = &node.options_from {
                validate_option_source(source)?;
            }
            Ok(())
        }
        PluginUiNodeType::Range => {
            let bind = node
                .bind
                .as_deref()
                .ok_or(UiValidationError::MissingField("bind", node.node_type))?;
            if !is_valid_binding(bind) {
                return Err(UiValidationError::InvalidBinding(bind.to_string()));
            }
            if let Some(step) = node.step {
                if !step.is_finite() || step <= 0.0 {
                    return Err(UiValidationError::InvalidRange);
                }
            }
            for bound in [node.min, node.max].into_iter().flatten() {
                if !bound.is_finite() {
                    return Err(UiValidationError::InvalidRange);
                }
            }
            if let (Some(min), Some(max)) = (node.min, node.max) {
                if min > max {
                    return Err(UiValidationError::InvalidRange);
                }
            }
            Ok(())
        }
        PluginUiNodeType::Checkbox => {
            let bind = node
                .bind
                .as_deref()
                .ok_or(UiValidationError::MissingField("bind", node.node_type))?;
            if !is_valid_binding(bind) {
                return Err(UiValidationError::InvalidBinding(bind.to_string()));
            }
            Ok(())
        }
        PluginUiNodeType::Button => {
            let action = node
                .action
                .as_ref()
                .ok_or(UiValidationError::InvalidAction(node.node_type))?;
            validate_action(action)?;
            if let Some(variant) = &node.variant {
                if variant != "primary" && variant != "danger" {
                    return Err(UiValidationError::InvalidVariant);
                }
            }
            Ok(())
        }
        PluginUiNodeType::List => {
            let source = node
                .options_from
                .as_deref()
                .ok_or(UiValidationError::MissingField(
                    "optionsFrom",
                    node.node_type,
                ))?;
            validate_option_source(source)
        }
        PluginUiNodeType::Separator | PluginUiNodeType::Connection => Ok(()),
        PluginUiNodeType::TtsSettings => {
            let contribution =
                node.contribution
                    .as_deref()
                    .ok_or(UiValidationError::MissingField(
                        "contribution",
                        node.node_type,
                    ))?;
            if contribution.trim().is_empty() || contribution.len() > 64 {
                return Err(UiValidationError::MissingField(
                    "contribution",
                    node.node_type,
                ));
            }
            Ok(())
        }
    }
}

fn validate_action(action: &super::nodes::PluginUiAction) -> Result<(), UiValidationError> {
    use super::nodes::PluginUiAction as A;
    match action {
        A::SaveSettings | A::TestConnection => Ok(()),
        A::PluginAction {
            action_type,
            config,
        } => {
            if !is_valid_action_type(action_type) {
                return Err(UiValidationError::InvalidAction(PluginUiNodeType::Button));
            }
            if let Some(config) = config {
                if !is_plain_json(config) {
                    return Err(UiValidationError::InvalidAction(PluginUiNodeType::Button));
                }
            }
            Ok(())
        }
        A::RefreshSource { source } => validate_option_source(source),
        A::OpenMediaPicker { accept, target } => {
            if accept.as_ref().is_some_and(|s| s.len() > 128) {
                return Err(UiValidationError::InvalidAction(PluginUiNodeType::Button));
            }
            if target.as_ref().is_some_and(|s| s.len() > 128) {
                return Err(UiValidationError::InvalidAction(PluginUiNodeType::Button));
            }
            Ok(())
        }
    }
}

fn is_valid_action_type(value: &str) -> bool {
    if value.is_empty() || value.len() > 128 {
        return false;
    }
    let mut chars = value.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_lowercase()) {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-')
}

/// Config payloads must be plain JSON objects (no depth bombs).
fn is_plain_json(value: &Value) -> bool {
    fn walk(value: &Value, depth: usize) -> bool {
        if depth > 8 {
            return false;
        }
        match value {
            Value::Object(map) => {
                if map.len() > 64 {
                    return false;
                }
                map.values().all(|v| walk(v, depth + 1))
            }
            Value::Array(items) => items.len() <= 64 && items.iter().all(|v| walk(v, depth + 1)),
            Value::String(s) => s.len() <= 4_096,
            _ => true,
        }
    }
    value.is_object() && walk(value, 0)
}

/// Validates a full page body plus its header fields.
pub fn validate_ui_page(
    id: &str,
    title: &LocalizedText,
    body: &PluginUiNode,
) -> Result<(), UiValidationError> {
    if id.trim().is_empty() || id.len() > 128 {
        return Err(UiValidationError::InvalidPageId);
    }
    validate_localized(title).map_err(|_| UiValidationError::InvalidPageTitle)?;
    validate_ui_node(body, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::nodes::{PluginUiAction, SelectOption};

    fn text(default: &str) -> LocalizedText {
        LocalizedText {
            default: default.to_string(),
            i18key: String::new(),
        }
    }

    fn node(node_type: PluginUiNodeType) -> PluginUiNode {
        PluginUiNode {
            node_type,
            key: None,
            label: None,
            title: None,
            children: None,
            text: None,
            schema: None,
            ui_hints: None,
            bind: None,
            options: None,
            options_from: None,
            min: None,
            max: None,
            step: None,
            action: None,
            variant: None,
            tone: None,
            contribution: None,
        }
    }

    #[test]
    fn accepts_a_nested_declarative_page() {
        let body = PluginUiNode {
            children: Some(vec![
                PluginUiNode {
                    text: Some(text("Hello")),
                    ..node(PluginUiNodeType::Text)
                },
                PluginUiNode {
                    bind: Some("settings.defaultVoice".to_string()),
                    options_from: Some(
                        "plugin-action-options:sonicboom.server.speak:voice".to_string(),
                    ),
                    label: Some(text("Default voice")),
                    ..node(PluginUiNodeType::Select)
                },
                PluginUiNode {
                    bind: Some("settings.volume".to_string()),
                    min: Some(0.0),
                    max: Some(1.0),
                    step: Some(0.01),
                    ..node(PluginUiNodeType::Range)
                },
                PluginUiNode {
                    action: Some(PluginUiAction::RefreshSource {
                        source: "plugin-action-options:a:b".to_string(),
                    }),
                    ..node(PluginUiNodeType::Button)
                },
                PluginUiNode {
                    contribution: Some("main".to_string()),
                    ..node(PluginUiNodeType::TtsSettings)
                },
            ]),
            ..node(PluginUiNodeType::Stack)
        };
        assert!(validate_ui_page("tts", &text("TTS"), &body).is_ok());
    }

    #[test]
    fn rejects_bad_bindings_ranges_and_actions() {
        let mut select = node(PluginUiNodeType::Select);
        select.bind = Some("eval(x)".to_string());
        assert_eq!(
            validate_ui_node(&select, 0),
            Err(UiValidationError::InvalidBinding("eval(x)".to_string()))
        );

        let mut proto = node(PluginUiNodeType::Range);
        proto.bind = Some("settings.__proto__.x".to_string());
        assert!(validate_ui_node(&proto, 0).is_err());

        let mut range = node(PluginUiNodeType::Range);
        range.bind = Some("settings.v".to_string());
        range.min = Some(5.0);
        range.max = Some(1.0);
        assert_eq!(
            validate_ui_node(&range, 0),
            Err(UiValidationError::InvalidRange)
        );

        // Unknown action types never deserialize into the allowlisted enum.
        let raw: Result<PluginUiAction, _> =
            serde_json::from_value(serde_json::json!({"type": "rpc", "method": "app.state.get"}));
        assert!(raw.is_err());

        let mut button = node(PluginUiNodeType::Button);
        button.action = Some(PluginUiAction::PluginAction {
            action_type: "../../etc/passwd".to_string(),
            config: None,
        });
        assert!(validate_ui_node(&button, 0).is_err());

        // Unknown node types never deserialize either.
        let raw: Result<PluginUiNode, _> =
            serde_json::from_value(serde_json::json!({"type": "obs-scene"}));
        assert!(raw.is_err());
    }

    #[test]
    fn enforces_depth_children_and_option_caps() {
        let mut deep = PluginUiNode {
            text: Some(text("leaf")),
            ..node(PluginUiNodeType::Text)
        };
        for _ in 0..12 {
            deep = PluginUiNode {
                children: Some(vec![deep]),
                ..node(PluginUiNodeType::Stack)
            };
        }
        assert_eq!(validate_ui_node(&deep, 0), Err(UiValidationError::TooDeep));

        let wide = PluginUiNode {
            children: Some(
                (0..40)
                    .map(|_| PluginUiNode {
                        text: Some(text("x")),
                        ..node(PluginUiNodeType::Text)
                    })
                    .collect(),
            ),
            ..node(PluginUiNodeType::Stack)
        };
        assert_eq!(
            validate_ui_node(&wide, 0),
            Err(UiValidationError::TooManyChildren)
        );

        let mut select = node(PluginUiNodeType::Select);
        select.bind = Some("settings.v".to_string());
        select.options = Some(
            (0..300)
                .map(|i| SelectOption {
                    value: format!("v{i}"),
                    label: text("x"),
                })
                .collect(),
        );
        assert_eq!(
            validate_ui_node(&select, 0),
            Err(UiValidationError::TooManyOptions)
        );
    }
}
