use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::{
    AudioPlayIntent, EmitIntent, EventEnrichmentResult, HostIntent, PluginCallResult, PluginEvent,
    PluginProtocolError, TextPronunciation, TextView,
};

/// Converts both the current typed result shape and the legacy `emit` /
/// `playAudio` keys into one internal contract. This is the single legacy
/// compatibility boundary used by the host after a runtime call.
pub fn decode_plugin_result(value: Value) -> Result<PluginCallResult, PluginProtocolError> {
    let object = value.as_object().ok_or(PluginProtocolError::NotAnObject)?;
    let summary = decode_summary(object.get("summary"))?;
    let logs = decode_logs(object.get("logs"))?;
    let mut intents = decode_typed_intents(object.get("intents"))?;
    intents.extend(decode_legacy_emit_intents(object.get("emit"))?);
    intents.extend(decode_legacy_audio_intents(object.get("playAudio"))?);
    let events = decode_events(object.get("events"))?;
    Ok(PluginCallResult {
        summary,
        logs,
        intents,
        events,
    })
}

fn decode_summary(value: Option<&Value>) -> Result<Option<String>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(None);
    };
    value
        .as_str()
        .map(|value| Some(value.to_owned()))
        .ok_or(PluginProtocolError::InvalidField("summary"))
}

fn decode_logs(value: Option<&Value>) -> Result<Vec<String>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    as_values(value)
        .into_iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or(PluginProtocolError::InvalidField("logs"))
        })
        .collect()
}

fn decode_typed_intents(value: Option<&Value>) -> Result<Vec<HostIntent>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    as_values(value)
        .into_iter()
        .map(|value| {
            serde_json::from_value(value.clone()).map_err(|error| {
                PluginProtocolError::InvalidValue {
                    field: "intents",
                    message: error.to_string(),
                }
            })
        })
        .collect()
}

fn decode_legacy_emit_intents(
    value: Option<&Value>,
) -> Result<Vec<HostIntent>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    as_values(value)
        .into_iter()
        .map(|value| {
            let object = value
                .as_object()
                .ok_or(PluginProtocolError::InvalidField("emit"))?;
            let event_type = object
                .get("type")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or(PluginProtocolError::InvalidField("emit"))?;
            Ok(HostIntent::Emit(EmitIntent::new(
                event_type,
                object.get("data").cloned().unwrap_or(Value::Null),
            )))
        })
        .collect()
}

fn decode_legacy_audio_intents(
    value: Option<&Value>,
) -> Result<Vec<HostIntent>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    as_values(value)
        .into_iter()
        .map(|value| {
            let mut object = value
                .as_object()
                .cloned()
                .ok_or(PluginProtocolError::InvalidField("playAudio"))?;
            if object.get("fileRef").is_some_and(Value::is_string) {
                let file = object
                    .remove("fileRef")
                    .ok_or(PluginProtocolError::InvalidField("playAudio"))?;
                object.insert("fileRef".to_owned(), serde_json::json!({"path": file}));
            } else if object.get("fileRef").is_none() {
                for key in ["filePath", "file", "path"] {
                    if let Some(file) = object.remove(key) {
                        object.insert(
                            "fileRef".to_owned(),
                            if file.is_string() {
                                serde_json::json!({"path": file})
                            } else {
                                file
                            },
                        );
                        break;
                    }
                }
            }
            let intent = serde_json::from_value::<AudioPlayIntent>(Value::Object(object)).map_err(
                |error| PluginProtocolError::InvalidValue {
                    field: "playAudio",
                    message: error.to_string(),
                },
            )?;
            Ok(HostIntent::AudioPlay(intent))
        })
        .collect()
}

fn decode_events(value: Option<&Value>) -> Result<Vec<PluginEvent>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    as_values(value)
        .into_iter()
        .map(|value| {
            serde_json::from_value(value.clone()).map_err(|error| {
                PluginProtocolError::InvalidValue {
                    field: "events",
                    message: error.to_string(),
                }
            })
        })
        .collect()
}

fn as_values(value: &Value) -> Vec<&Value> {
    match value {
        Value::Array(values) => values.iter().collect(),
        value => vec![value],
    }
}

/// Bounds for one enrichment result. Processors run per live event, so every
/// plugin-controlled collection and string accepted here is capped; the host
/// treats an oversized result as an invalid response and passes the raw
/// event through.
pub const MAX_ENRICHMENT_ANNOTATIONS: usize = 16;
pub const MAX_ENRICHMENT_ANNOTATION_BYTES: usize = 32 * 1024;
pub const MAX_ENRICHMENT_VIEWS: usize = 16;
pub const MAX_ENRICHMENT_VIEW_TEXT_CHARS: usize = 4_096;
pub const MAX_ENRICHMENT_LOGS: usize = 16;
pub const MAX_ENRICHMENT_LOG_CHARS: usize = 512;

/// Top-level keys that would smuggle side effects through the enrichment
/// channel. They are rejected, never executed or merged.
const FORBIDDEN_ENRICHMENT_KEYS: [&str; 8] = [
    "emit",
    "events",
    "intents",
    "playAudio",
    "audioPlay",
    "points",
    "http",
    "storage",
];

/// Decodes one `enrich` response into the constrained enrichment contract.
/// Unlike `decode_plugin_result` this accepts no legacy intent keys: any
/// side-effect field is a typed error so the host can fail open with a
/// precise diagnostic instead of executing it.
pub fn decode_enrichment_result(
    value: Value,
) -> Result<EventEnrichmentResult, PluginProtocolError> {
    let object = value.as_object().ok_or(PluginProtocolError::NotAnObject)?;
    for key in FORBIDDEN_ENRICHMENT_KEYS {
        if object.contains_key(key) {
            return Err(PluginProtocolError::InvalidValue {
                field: "enrich",
                message: format!("enrichment results must not contain `{key}`"),
            });
        }
    }
    let annotations = decode_enrichment_annotations(object.get("annotations"))?;
    let views = decode_enrichment_views(object.get("views"))?;
    let logs = decode_enrichment_logs(object.get("logs"))?;
    Ok(EventEnrichmentResult {
        annotations,
        views,
        logs,
    })
}

fn decode_enrichment_annotations(
    value: Option<&Value>,
) -> Result<Map<String, Value>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Map::new());
    };
    let object = value
        .as_object()
        .ok_or(PluginProtocolError::InvalidField("annotations"))?;
    if object.len() > MAX_ENRICHMENT_ANNOTATIONS {
        return Err(PluginProtocolError::InvalidValue {
            field: "annotations",
            message: format!("at most {MAX_ENRICHMENT_ANNOTATIONS} annotations are allowed"),
        });
    }
    for key in object.keys() {
        if key.is_empty() || key.len() > 64 {
            return Err(PluginProtocolError::InvalidValue {
                field: "annotations",
                message: "annotation names must be 1..=64 characters".to_owned(),
            });
        }
    }
    if serde_json::to_vec(&object)
        .map(|bytes| bytes.len() > MAX_ENRICHMENT_ANNOTATION_BYTES)
        .unwrap_or(true)
    {
        return Err(PluginProtocolError::InvalidValue {
            field: "annotations",
            message: format!("annotations are larger than {MAX_ENRICHMENT_ANNOTATION_BYTES} bytes"),
        });
    }
    Ok(object.clone())
}

fn decode_enrichment_views(
    value: Option<&Value>,
) -> Result<BTreeMap<String, TextView>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let object = value
        .as_object()
        .ok_or(PluginProtocolError::InvalidField("views"))?;
    if object.len() > MAX_ENRICHMENT_VIEWS {
        return Err(PluginProtocolError::InvalidValue {
            field: "views",
            message: format!("at most {MAX_ENRICHMENT_VIEWS} views are allowed"),
        });
    }
    let mut views = BTreeMap::new();
    for (name, view) in object {
        if name.is_empty() || name.len() > 64 {
            return Err(PluginProtocolError::InvalidValue {
                field: "views",
                message: "view names must be 1..=64 characters".to_owned(),
            });
        }
        let view: TextView = serde_json::from_value(view.clone()).map_err(|error| {
            PluginProtocolError::InvalidValue {
                field: "views",
                message: error.to_string(),
            }
        })?;
        if view.text.chars().count() > MAX_ENRICHMENT_VIEW_TEXT_CHARS {
            return Err(PluginProtocolError::InvalidValue {
                field: "views",
                message: format!(
                    "view text is longer than {MAX_ENRICHMENT_VIEW_TEXT_CHARS} characters"
                ),
            });
        }
        if view.source.is_empty() || view.source.len() > 64 {
            return Err(PluginProtocolError::InvalidValue {
                field: "views",
                message: "view source must be 1..=64 characters".to_owned(),
            });
        }
        if view
            .language
            .as_ref()
            .is_some_and(|language| language.is_empty() || language.len() > 32)
        {
            return Err(PluginProtocolError::InvalidValue {
                field: "views",
                message: "view language must be 1..=32 characters".to_owned(),
            });
        }
        if view
            .confidence
            .is_some_and(|confidence| !confidence.is_finite() || !(0.0..=1.0).contains(&confidence))
        {
            return Err(PluginProtocolError::InvalidValue {
                field: "views",
                message: "view confidence must be within 0.0..=1.0".to_owned(),
            });
        }
        if view
            .reason
            .as_ref()
            .is_some_and(|reason| reason.len() > 128)
        {
            return Err(PluginProtocolError::InvalidValue {
                field: "views",
                message: "view reason is longer than 128 characters".to_owned(),
            });
        }
        if let Some(pronunciation) = &view.pronunciation {
            decode_view_pronunciation(pronunciation)?;
        }
        views.insert(name.clone(), view);
    }
    Ok(views)
}

fn decode_view_pronunciation(pronunciation: &TextPronunciation) -> Result<(), PluginProtocolError> {
    if pronunciation
        .ipa
        .as_ref()
        .is_some_and(|ipa| ipa.len() > 256)
    {
        return Err(PluginProtocolError::InvalidValue {
            field: "views",
            message: "view ipa is longer than 256 characters".to_owned(),
        });
    }
    if pronunciation
        .language
        .as_ref()
        .is_some_and(|language| language.is_empty() || language.len() > 32)
    {
        return Err(PluginProtocolError::InvalidValue {
            field: "views",
            message: "view pronunciation language must be 1..=32 characters".to_owned(),
        });
    }
    if pronunciation
        .dialect
        .as_ref()
        .is_some_and(|dialect| dialect.is_empty() || dialect.len() > 32)
    {
        return Err(PluginProtocolError::InvalidValue {
            field: "views",
            message: "view pronunciation dialect must be 1..=32 characters".to_owned(),
        });
    }
    if pronunciation
        .confidence
        .is_some_and(|confidence| !confidence.is_finite() || !(0.0..=1.0).contains(&confidence))
    {
        return Err(PluginProtocolError::InvalidValue {
            field: "views",
            message: "view pronunciation confidence must be within 0.0..=1.0".to_owned(),
        });
    }
    Ok(())
}

fn decode_enrichment_logs(value: Option<&Value>) -> Result<Vec<String>, PluginProtocolError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    as_values(value)
        .into_iter()
        .take(MAX_ENRICHMENT_LOGS)
        .map(|value| {
            value
                .as_str()
                .map(|line| line.chars().take(MAX_ENRICHMENT_LOG_CHARS).collect())
                .ok_or(PluginProtocolError::InvalidField("logs"))
        })
        .collect()
}
