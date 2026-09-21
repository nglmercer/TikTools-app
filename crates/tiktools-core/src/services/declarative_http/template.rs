//! Scoped template rendering for declarative requests.

use crate::helpers::value_to_string;
use serde_json::Value;

/// Renders `{{ dotted.path }}` templates against a scope object such as
/// `{"event": …, "settings": …, "config": …}`. Unknown paths render empty,
/// matching the automation template behavior.
pub(crate) fn render_scoped_template(source: &str, scope: &Value) -> String {
    let mut rendered = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("{{") {
        rendered.push_str(&rest[..start]);
        let expression = &rest[start + 2..];
        let Some(end) = expression.find("}}") else {
            rendered.push_str(&rest[start..]);
            return rendered;
        };
        let path = expression[..end].trim();
        if let Some(value) = read_scope_path(scope, path) {
            rendered.push_str(&value_to_string(value));
        }
        rest = &expression[end + 2..];
    }
    rendered.push_str(rest);
    rendered
}

fn read_scope_path<'a>(scope: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = scope;
    for part in path.split('.') {
        if part.is_empty() {
            continue;
        }
        current = current.get(part)?;
    }
    Some(current)
}
