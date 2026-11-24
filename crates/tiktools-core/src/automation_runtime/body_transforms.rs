//! Ordered text transforms applied to a rendered HTTP request body.
//!
//! Both declarative plugin actions (`http.bodyTransforms` in the manifest)
//! and user `action.http` nodes (`bodyTransforms` in the action config)
//! share this pipeline: every string parameter is template-rendered first,
//! then the ops run in order over the body. The op set stays deliberately
//! small and bounded — unknown ops and invalid regexes fail the action
//! instead of silently shipping untransformed text.

use serde_json::Value;

/// Most transforms one request may declare.
pub(crate) const MAX_BODY_TRANSFORMS: usize = 8;
/// Rendered transform parameters stay bounded like other untrusted text.
const MAX_TRANSFORM_PARAM_CHARS: usize = 1024;

const OPS: &[&str] = &[
    "strip-prefix",
    "strip-suffix",
    "trim",
    "replace",
    "regex-replace",
];

/// Applies the `bodyTransforms` array to an already-rendered body.
/// `render` resolves one template source (event/globals scope for user
/// nodes; event/globals after the declarative config pass for plugin
/// actions). A missing or empty array is a no-op; malformed specs error.
pub(crate) fn apply_body_transforms(
    body: &str,
    transforms: &Value,
    render: &dyn Fn(&str) -> String,
) -> Result<String, String> {
    let items = transforms
        .as_array()
        .ok_or_else(|| "bodyTransforms must be an array.".to_owned())?;
    if items.len() > MAX_BODY_TRANSFORMS {
        return Err(format!(
            "bodyTransforms allows at most {MAX_BODY_TRANSFORMS} entries."
        ));
    }
    let mut body = body.to_owned();
    for (index, item) in items.iter().enumerate() {
        let op = item
            .as_object()
            .ok_or_else(|| format!("bodyTransforms[{index}] must be an object."))?;
        let kind = op.get("op").and_then(Value::as_str).unwrap_or_default();
        if !OPS.contains(&kind) {
            return Err(format!(
                "bodyTransforms[{index}] has an unknown op `{kind}`."
            ));
        }
        let param = |key: &str| -> Result<String, String> {
            let raw = op
                .get(key)
                .ok_or_else(|| format!("bodyTransforms[{index}] `{kind}` requires `{key}`."))?;
            let rendered = render(&crate::helpers::value_to_string(raw));
            if rendered.chars().count() > MAX_TRANSFORM_PARAM_CHARS {
                return Err(format!(
                    "bodyTransforms[{index}] `{key}` exceeds {MAX_TRANSFORM_PARAM_CHARS} characters."
                ));
            }
            Ok(rendered)
        };
        let optional = |key: &str| -> String {
            op.get(key)
                .map(|raw| render(&crate::helpers::value_to_string(raw)))
                .unwrap_or_default()
        };
        body = match kind {
            "strip-prefix" => strip_affix(&body, &param("value")?, true),
            "strip-suffix" => strip_affix(&body, &param("value")?, false),
            "trim" => match optional("side").as_str() {
                "" | "both" => body.trim().to_owned(),
                "start" => body.trim_start().to_owned(),
                "end" => body.trim_end().to_owned(),
                side => {
                    return Err(format!(
                    "bodyTransforms[{index}] trim side must be start, end or both (got `{side}`)."
                ))
                }
            },
            "replace" => {
                let find = param("find")?;
                if find.is_empty() {
                    body
                } else {
                    body.replace(&find, &optional("with"))
                }
            }
            "regex-replace" => {
                let pattern = param("pattern")?;
                let regex = regex::Regex::new(&pattern).map_err(|_| {
                    format!("bodyTransforms[{index}] regex-replace has an invalid pattern.")
                })?;
                regex
                    .replace_all(&body, optional("with").as_str())
                    .into_owned()
            }
            _ => unreachable!("op list and match arms stay in sync"),
        };
    }
    Ok(body)
}

/// Removes `affix` from one end of the body, ASCII case-insensitively like
/// the filter `starts-with`/`ends-with` semantics, plus the whitespace that
/// separated it from the payload. An empty or non-matching affix is a no-op.
fn strip_affix(body: &str, affix: &str, prefix: bool) -> String {
    let affix = affix.trim();
    if affix.is_empty() {
        return body.to_owned();
    }
    let edge = if prefix {
        body.get(..affix.len())
    } else {
        body.get(body.len().saturating_sub(affix.len())..)
    };
    let Some(edge) = edge else {
        return body.to_owned();
    };
    if !edge.eq_ignore_ascii_case(affix) {
        return body.to_owned();
    }
    if prefix {
        body[affix.len()..].trim_start().to_owned()
    } else {
        body[..body.len() - affix.len()].trim_end().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn identity(source: &str) -> String {
        source.to_owned()
    }

    fn apply(body: &str, transforms: Value) -> Result<String, String> {
        apply_body_transforms(body, &transforms, &identity)
    }

    #[test]
    fn strip_prefix_removes_command_case_insensitively() {
        let transforms = json!([{"op": "strip-prefix", "value": "!voice1"}]);
        assert_eq!(apply("!voice1 hola", transforms.clone()).unwrap(), "hola");
        assert_eq!(apply("!VOICE1   hola", transforms.clone()).unwrap(), "hola");
        assert_eq!(apply("!voice1", transforms.clone()).unwrap(), "");
        assert_eq!(apply("!voice10 hola", transforms).unwrap(), "0 hola");
        assert_eq!(
            apply("hola", json!([{"op": "strip-prefix", "value": "!voice1"}])).unwrap(),
            "hola"
        );
        assert_eq!(
            apply(
                "!voice1 hola",
                json!([{"op": "strip-prefix", "value": "  "}])
            )
            .unwrap(),
            "!voice1 hola"
        );
    }

    #[test]
    fn strip_suffix_and_trim_compose_in_order() {
        let transforms = json!([
            {"op": "strip-suffix", "value": "!"},
            {"op": "trim", "side": "both"},
        ]);
        assert_eq!(apply("  hola !", transforms).unwrap(), "hola");
        assert_eq!(
            apply("x", json!([{"op": "trim", "side": "start"}])).unwrap(),
            "x"
        );
        assert!(apply("x", json!([{"op": "trim", "side": "middle"}])).is_err());
    }

    #[test]
    fn replace_is_literal_and_replace_all() {
        let transforms = json!([{"op": "replace", "find": "btw", "with": "by the way"}]);
        assert_eq!(
            apply("btw this btw that", transforms).unwrap(),
            "by the way this by the way that"
        );
        assert_eq!(
            apply("aaa", json!([{"op": "replace", "find": "", "with": "x"}])).unwrap(),
            "aaa"
        );
    }

    #[test]
    fn regex_replace_supports_classes_and_capture_refs() {
        let transforms = json!([{"op": "regex-replace", "pattern": "^!\\w+\\s*", "with": ""}]);
        assert_eq!(apply("!voice1 hola", transforms).unwrap(), "hola");
        let transforms =
            json!([{"op": "regex-replace", "pattern": "(\\w+)@(\\w+)", "with": "$2@$1"}]);
        assert_eq!(apply("mail a@b", transforms).unwrap(), "mail b@a");
        assert!(apply(
            "x",
            json!([{"op": "regex-replace", "pattern": "([", "with": ""}])
        )
        .is_err());
    }

    #[test]
    fn malformed_specs_fail_closed() {
        assert!(apply("x", json!({})).is_err());
        assert!(apply("x", json!([{"op": "nope"}])).is_err());
        assert!(apply("x", json!([{"op": "replace"}])).is_err());
        let many: Vec<Value> = (0..9).map(|_| json!({"op": "trim"})).collect();
        assert!(apply("x", json!(many)).is_err());
    }
}
