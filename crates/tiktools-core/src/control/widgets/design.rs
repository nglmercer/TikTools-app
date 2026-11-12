//! Portable widget design projection: known tokens and bounded content only.

/// Copy only known style tokens, never arbitrary application state or CSS.
/// Mirrors the WebView `normalizeDesign` bounds so OBS renders exactly what
/// the builder preview shows.
pub(super) fn portable_design(raw: &str) -> Option<String> {
    let source: serde_json::Value = serde_json::from_str(raw).ok()?;
    let mut design = serde_json::Map::new();
    copy_text(&source, &mut design);
    copy_layers(&source, &mut design);
    copy_card_style(&source, &mut design);
    copy_avatar(&source, &mut design);
    copy_badge(&source, &mut design);
    (!design.is_empty()).then(|| serde_json::Value::Object(design).to_string())
}

fn portable_color(value: Option<&serde_json::Value>) -> Option<&str> {
    let color = value.and_then(|value| value.as_str())?;
    (matches!(color.len(), 7 | 9)
        && color.starts_with('#')
        && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit))
    .then_some(color)
}

fn portable_number(value: Option<&serde_json::Value>, min: f64, max: f64) -> Option<f64> {
    value.and_then(|value| value.as_f64()).map(|number| {
        if number.is_finite() {
            number.clamp(min, max)
        } else {
            min
        }
    })
}

const TEXT_FIELDS: [&str; 7] = [
    "title",
    "streakTitle",
    "name",
    "handle",
    "message",
    "count",
    "diamonds",
];

fn copy_text(source: &serde_json::Value, design: &mut serde_json::Map<String, serde_json::Value>) {
    if let Some(fields) = source.get("text").and_then(|value| value.as_object()) {
        let mut text = serde_json::Map::new();
        for key in TEXT_FIELDS {
            if let Some(value) = fields.get(key).and_then(|value| value.as_str()) {
                text.insert(
                    key.to_owned(),
                    serde_json::Value::from(value.chars().take(300).collect::<String>()),
                );
            }
        }
        if !text.is_empty() {
            design.insert("text".to_owned(), serde_json::Value::Object(text));
        }
    }
    for key in ["hiddenText", "textOrder"] {
        if let Some(fields) = source.get(key).and_then(|value| value.as_array()) {
            let mut kept: Vec<serde_json::Value> = Vec::new();
            for field in fields {
                let Some(name) = field.as_str() else {
                    continue;
                };
                if !TEXT_FIELDS.contains(&name)
                    || kept.iter().any(|kept| kept.as_str() == Some(name))
                {
                    continue;
                }
                kept.push(serde_json::Value::from(name));
                if kept.len() >= TEXT_FIELDS.len() {
                    break;
                }
            }
            if !kept.is_empty() {
                design.insert(key.to_owned(), serde_json::Value::Array(kept));
            }
        }
    }
}

fn copy_layers(
    source: &serde_json::Value,
    design: &mut serde_json::Map<String, serde_json::Value>,
) {
    if let Some(layers) = source.get("layers").and_then(|value| value.as_array()) {
        let mut kept = Vec::new();
        let mut ids = std::collections::HashSet::new();
        for item in layers {
            let Some(layer) = item.as_object() else {
                continue;
            };
            let Some(id) = layer.get("id").and_then(|value| value.as_str()) else {
                continue;
            };
            let valid_id = !id.is_empty()
                && id.len() <= 80
                && id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-'));
            if !valid_id || ids.contains(id) {
                continue;
            }
            let Some(kind) = layer.get("kind").and_then(|value| value.as_str()) else {
                continue;
            };
            if !matches!(kind, "text" | "avatar" | "art") {
                continue;
            }
            ids.insert(id.to_owned());
            let mut entry = serde_json::Map::new();
            entry.insert("id".to_owned(), serde_json::Value::from(id));
            entry.insert("kind".to_owned(), serde_json::Value::from(kind));
            let name = layer
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or(id);
            entry.insert(
                "name".to_owned(),
                serde_json::Value::from(name.chars().take(80).collect::<String>()),
            );
            if kind == "text" {
                let text = layer
                    .get("text")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                entry.insert(
                    "text".to_owned(),
                    serde_json::Value::from(text.chars().take(300).collect::<String>()),
                );
                if let Some(field) = layer.get("field").and_then(|value| value.as_str()) {
                    if TEXT_FIELDS.contains(&field) {
                        entry.insert("field".to_owned(), serde_json::Value::from(field));
                    }
                }
                if let Some(color) = portable_color(layer.get("color")) {
                    entry.insert("color".to_owned(), serde_json::Value::from(color));
                }
                if let Some(size) = portable_number(layer.get("fontSize"), 8.0, 96.0) {
                    entry.insert("fontSize".to_owned(), serde_json::Value::from(size));
                }
                if let Some(weight) = portable_number(layer.get("fontWeight"), 100.0, 900.0) {
                    entry.insert(
                        "fontWeight".to_owned(),
                        serde_json::Value::from((weight / 100.0).round() * 100.0),
                    );
                }
            } else {
                if let Some(size) = portable_number(layer.get("size"), 16.0, 160.0) {
                    entry.insert("size".to_owned(), serde_json::Value::from(size));
                }
                if let Some(placement) = layer.get("placement").and_then(|value| value.as_str()) {
                    if matches!(placement, "left" | "right" | "top" | "bottom") {
                        entry.insert("placement".to_owned(), serde_json::Value::from(placement));
                    }
                }
            }
            kept.push(serde_json::Value::Object(entry));
        }
        design.insert("layers".to_owned(), serde_json::Value::Array(kept));
    }
}

fn copy_card_style(
    source: &serde_json::Value,
    design: &mut serde_json::Map<String, serde_json::Value>,
) {
    for key in ["background", "textColor", "accent", "borderColor"] {
        if let Some(color) = portable_color(source.get(key)) {
            design.insert(key.to_owned(), serde_json::Value::from(color));
        }
    }
    for (key, min, max) in [
        ("radius", 0.0, 48.0),
        ("borderWidth", 0.0, 8.0),
        ("opacity", 0.0, 100.0),
        ("padding", 0.0, 64.0),
        ("gap", 0.0, 48.0),
        ("width", 240.0, 720.0),
    ] {
        if let Some(number) = portable_number(source.get(key), min, max) {
            design.insert(key.to_owned(), serde_json::Value::from(number));
        }
    }
    if let Some(shadow) = source.get("shadow").and_then(|value| value.as_bool()) {
        design.insert("shadow".to_owned(), serde_json::Value::from(shadow));
    }
    if let Some(align) = source.get("align").and_then(|value| value.as_str()) {
        if matches!(align, "left" | "center" | "right") {
            design.insert("align".to_owned(), serde_json::Value::from(align));
        }
    }
}

fn copy_avatar(
    source: &serde_json::Value,
    design: &mut serde_json::Map<String, serde_json::Value>,
) {
    if let Some(avatar) = source.get("avatar").and_then(|value| value.as_object()) {
        let mut kept = serde_json::Map::new();
        if let Some(visible) = avatar.get("visible").and_then(|value| value.as_bool()) {
            kept.insert("visible".to_owned(), serde_json::Value::from(visible));
        }
        for (key, min, max) in [
            ("size", 16.0, 160.0),
            ("radius", 0.0, 80.0),
            ("borderWidth", 0.0, 8.0),
        ] {
            if let Some(number) = portable_number(avatar.get(key), min, max) {
                kept.insert(key.to_owned(), serde_json::Value::from(number));
            }
        }
        if let Some(color) = portable_color(avatar.get("borderColor")) {
            kept.insert("borderColor".to_owned(), serde_json::Value::from(color));
        }
        if !kept.is_empty() {
            design.insert("avatar".to_owned(), serde_json::Value::Object(kept));
        }
    }
}

fn copy_badge(source: &serde_json::Value, design: &mut serde_json::Map<String, serde_json::Value>) {
    if let Some(badge) = source.get("badge").and_then(|value| value.as_object()) {
        let mut kept = serde_json::Map::new();
        if let Some(visible) = badge.get("visible").and_then(|value| value.as_bool()) {
            kept.insert("visible".to_owned(), serde_json::Value::from(visible));
        }
        if let Some(color) = portable_color(badge.get("color")) {
            kept.insert("color".to_owned(), serde_json::Value::from(color));
        }
        for (key, min, max) in [("fontSize", 8.0, 32.0), ("letterSpacing", 0.0, 8.0)] {
            if let Some(number) = portable_number(badge.get(key), min, max) {
                kept.insert(key.to_owned(), serde_json::Value::from(number));
            }
        }
        if let Some(weight) = badge.get("fontWeight").and_then(|value| value.as_f64()) {
            let rounded = (weight / 100.0).round() * 100.0;
            kept.insert(
                "fontWeight".to_owned(),
                serde_json::Value::from(rounded.clamp(400.0, 900.0)),
            );
        }
        if !kept.is_empty() {
            design.insert("badge".to_owned(), serde_json::Value::Object(kept));
        }
    }
}
