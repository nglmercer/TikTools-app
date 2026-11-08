//! Measure resolved source before cloning params or allocating expanded text.
use super::*;
use std::io::{self, Write};

const RECORD_BYTES: usize = 256 * 1024;
const IMPORT_BYTES: usize = 8 * 1024 * 1024;
const MAX_NODES: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemplateInstantiationError {
    TooLarge,
}
impl std::fmt::Display for TemplateInstantiationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("expanded template exceeds its source byte or structure budget")
    }
}
impl std::error::Error for TemplateInstantiationError {}
type Result<T> = std::result::Result<T, TemplateInstantiationError>;
fn too_large() -> TemplateInstantiationError {
    TemplateInstantiationError::TooLarge
}

/// Reuse one budget across every entry in an import or dry run. Charges the
/// serialized entry-array bytes; a failed entry consumes none of the budget.
#[derive(Clone, Debug)]
pub struct TemplateInstantiationBudget {
    limit: usize,
    used: usize,
    entries: usize,
}
impl Default for TemplateInstantiationBudget {
    fn default() -> Self {
        Self::new()
    }
}
impl TemplateInstantiationBudget {
    pub fn new() -> Self {
        Self::with_limit(IMPORT_BYTES)
    }
    pub fn with_limit(limit: usize) -> Self {
        Self {
            limit: limit.min(IMPORT_BYTES),
            used: 2,
            entries: 0,
        }
    }
    pub fn used_bytes(&self) -> usize {
        self.used
    }
}

#[derive(Default)]
struct Measure {
    bytes: usize,
    nodes: usize,
}
impl Measure {
    fn add(&mut self, bytes: usize) -> Result<()> {
        self.bytes = self.bytes.checked_add(bytes).ok_or_else(too_large)?;
        if self.bytes > RECORD_BYTES {
            return Err(too_large());
        }
        Ok(())
    }
    fn node(&mut self, depth: usize) -> Result<()> {
        self.nodes += 1;
        if depth > 64 || self.nodes > MAX_NODES {
            return Err(too_large());
        }
        Ok(())
    }
    fn fragment(&mut self, text: &[u8]) -> Result<()> {
        for byte in text {
            self.add(match byte {
                b'"' | b'\\' | b'\n' | b'\r' | b'\t' | 8 | 12 => 2,
                0..=31 => 6,
                _ => 1,
            })?;
        }
        Ok(())
    }
    fn string(&mut self, text: &str, depth: usize) -> Result<()> {
        self.node(depth)?;
        self.add(2)?;
        self.fragment(text.as_bytes())
    }
    fn key(&mut self, key: &str) -> Result<()> {
        self.add(3)?;
        self.fragment(key.as_bytes())
    }
    fn object(&mut self, depth: usize, count: usize) -> Result<()> {
        self.node(depth)?;
        self.add(2usize.saturating_add(count.saturating_sub(1)))
    }
    fn array(&mut self, depth: usize, count: usize) -> Result<()> {
        self.node(depth)?;
        self.add(2usize.saturating_add(count.saturating_sub(1)))
    }
    fn raw(&mut self, value: &Value, depth: usize) -> Result<()> {
        match value {
            Value::Object(map) => self.map(map, depth, None),
            Value::Array(items) => {
                self.array(depth, items.len())?;
                for item in items {
                    self.raw(item, depth + 1)?;
                }
                Ok(())
            }
            Value::String(text) => self.string(text, depth),
            other => {
                self.node(depth)?;
                self.add(match other {
                    Value::Null => 4,
                    Value::Bool(true) => 4,
                    Value::Bool(false) => 5,
                    Value::Number(number) => number.to_string().len(),
                    _ => unreachable!(),
                })
            }
        }
    }
    fn map(
        &mut self,
        map: &Map<String, Value>,
        depth: usize,
        params: Option<&Map<String, Value>>,
    ) -> Result<()> {
        self.object(depth, map.len())?;
        for (key, value) in map {
            self.key(key)?;
            if let Some(params) = params {
                self.substitute(value, params, depth + 1)?;
            } else {
                self.raw(value, depth + 1)?;
            }
        }
        Ok(())
    }
    fn substitute(
        &mut self,
        value: &Value,
        params: &Map<String, Value>,
        depth: usize,
    ) -> Result<()> {
        match value {
            Value::String(text) => self.text(text, params, depth),
            Value::Object(map) => self.map(map, depth, Some(params)),
            Value::Array(items) => {
                self.array(depth, items.len())?;
                for item in items {
                    self.substitute(item, params, depth + 1)?;
                }
                Ok(())
            }
            _ => self.raw(value, depth),
        }
    }
    fn text(&mut self, text: &str, params: &Map<String, Value>, depth: usize) -> Result<()> {
        if let Some(path) = sole_params_span(text) {
            return match read_param_path_ref(params, &path) {
                Some(value) => self.raw(value, depth),
                None => self.string("", depth),
            };
        }
        self.node(depth)?;
        self.add(2)?;
        let mut cursor = 0;
        while let Some(open) = text[cursor..].find("{{") {
            let start = cursor + open;
            if let Some((end, path)) = scan_params_span(text, start) {
                self.fragment(&text.as_bytes()[cursor..start])?;
                self.scalar(read_param_path_ref(params, &path))?;
                cursor = end;
            } else {
                self.fragment(&text.as_bytes()[cursor..start + 2])?;
                cursor = start + 2;
            }
        }
        self.fragment(&text.as_bytes()[cursor..])
    }
    fn scalar(&mut self, value: Option<&Value>) -> Result<()> {
        match value {
            None | Some(Value::Null) => Ok(()),
            Some(Value::String(text)) => self.fragment(text.as_bytes()),
            Some(Value::Number(number)) => self.fragment(number.to_string().as_bytes()),
            Some(Value::Bool(flag)) => self.fragment(if *flag { b"true" } else { b"false" }),
            Some(value) => {
                guard_structure(value)?;
                serde_json::to_writer(EscapedCounter(self), value).map_err(|_| too_large())
            }
        }
    }
    fn name(
        &mut self,
        fallback: &str,
        override_name: Option<&str>,
        params: &Map<String, Value>,
        depth: usize,
    ) -> Result<()> {
        if let Some(name) = override_name.map(str::trim).filter(|name| !name.is_empty()) {
            return self.string(name, depth);
        }
        if let Some(path) = sole_params_span(fallback) {
            return self.string(
                read_param_path_ref(params, &path)
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                depth,
            );
        }
        self.text(fallback, params, depth)
    }
}
struct EscapedCounter<'a>(&'a mut Measure);
impl Write for EscapedCounter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .fragment(bytes)
            .map_err(|_| io::Error::other("template expansion budget"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn guard_structure(value: &Value) -> Result<()> {
    let mut pending = vec![(value, 0usize)];
    let mut nodes = 0;
    while let Some((value, depth)) = pending.pop() {
        nodes += 1;
        if nodes > MAX_NODES || depth > 64 {
            return Err(too_large());
        }
        let children: Box<dyn Iterator<Item = &Value> + '_> = match value {
            Value::Object(map) => Box::new(map.values()),
            Value::Array(items) => Box::new(items.iter()),
            _ => continue,
        };
        for child in children {
            if nodes + pending.len() >= MAX_NODES {
                return Err(too_large());
            }
            pending.push((child, depth + 1));
        }
    }
    Ok(())
}
fn measure_action(
    action: &RuleTemplateAction,
    index: usize,
    id: &str,
    params: &Map<String, Value>,
    names: &[String],
) -> Result<usize> {
    let mut m = Measure::default();
    m.object(0, 6)?;
    m.key("schemaVersion")?;
    m.raw(&Value::from(2), 1)?;
    m.key("id")?;
    m.string(id, 1)?;
    m.key("name")?;
    m.name(
        &action.name,
        names.get(index).map(String::as_str),
        params,
        1,
    )?;
    m.key("typeId")?;
    m.string(&action.type_id, 1)?;
    m.key("enabled")?;
    m.raw(&Value::Bool(action.enabled), 1)?;
    m.key("config")?;
    m.map(&action.config, 1, Some(params))?;
    Ok(m.bytes)
}
fn measure_event(
    event: &RuleTemplateEvent,
    id: &str,
    ids: &[String],
    params: &Map<String, Value>,
    name: Option<&str>,
) -> Result<usize> {
    let mut m = Measure::default();
    m.object(0, 10)?;
    m.key("schemaVersion")?;
    m.raw(&Value::from(1), 1)?;
    m.key("id")?;
    m.string(id, 1)?;
    m.key("name")?;
    m.name(&event.name, name, params, 1)?;
    m.key("enabled")?;
    m.raw(&Value::Bool(event.enabled), 1)?;
    m.key("trigger")?;
    m.string(&event.trigger, 1)?;
    m.key("filters")?;
    m.array(1, event.filters.len())?;
    for filter in &event.filters {
        m.object(2, if filter.values.is_some() { 4 } else { 3 })?;
        for (key, text) in [
            ("path", &filter.path),
            ("operator", &filter.operator),
            ("value", &filter.value),
        ] {
            m.key(key)?;
            m.text(text, params, 3)?;
        }
        if let Some(values) = &filter.values {
            m.key("values")?;
            m.array(3, values.len())?;
            for value in values {
                m.text(value, params, 4)?;
            }
        }
    }
    m.key("cooldownMs")?;
    m.raw(&Value::from(event.cooldown_ms), 1)?;
    m.key("cooldownScope")?;
    m.string(&event.cooldown_scope, 1)?;
    m.key("actionIds")?;
    m.array(1, ids.len())?;
    for id in ids {
        m.string(id, 2)?;
    }
    m.key("runMode")?;
    m.string(&event.run_mode, 1)?;
    Ok(m.bytes)
}

/// Preserve ordinary substitution semantics while admitting the complete
/// expanded entry before building any resolved string or cloned parameter tree.
pub fn instantiate_template_bounded(
    template: &RuleTemplate,
    params: &Map<String, Value>,
    event_name: Option<&str>,
    action_names: &[String],
    budget: &mut TemplateInstantiationBudget,
) -> Result<InstantiatedRule> {
    if template.actions.is_empty()
        || template.actions.len() > MAX_ACTIONS
        || template.event.filters.len() > MAX_FILTERS
    {
        return Err(too_large());
    }
    let ids: Vec<String> = template
        .actions
        .iter()
        .map(|_| fresh_record_id("act"))
        .collect();
    let event_id = fresh_record_id("evt");
    let mut bytes =
        b"{\"actions\":[".len() + b"],\"event\":".len() + 1 + ids.len().saturating_sub(1);
    for (index, action) in template.actions.iter().enumerate() {
        bytes = bytes
            .checked_add(measure_action(
                action,
                index,
                &ids[index],
                params,
                action_names,
            )?)
            .ok_or_else(too_large)?;
    }
    bytes = bytes
        .checked_add(measure_event(
            &template.event,
            &event_id,
            &ids,
            params,
            event_name,
        )?)
        .ok_or_else(too_large)?;
    let used = budget
        .used
        .checked_add(bytes)
        .and_then(|size| size.checked_add(usize::from(budget.entries > 0)))
        .ok_or_else(too_large)?;
    if used > budget.limit {
        return Err(too_large());
    }
    let result =
        instantiate_template_with_ids(template, params, event_name, action_names, &ids, &event_id);
    budget.used = used;
    budget.entries += 1;
    Ok(result)
}

#[cfg(test)]
mod tests;
