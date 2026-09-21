//! CLI argument parsing helpers.

use std::collections::BTreeMap;

use serde_json::Value;

pub fn split_first<'a>(args: &'a [String], usage: &str) -> Result<(&'a str, &'a [String]), String> {
    args.split_first()
        .map(|(first, rest)| (first.as_str(), rest))
        .ok_or_else(|| usage.to_owned())
}

pub fn one_arg(args: &[String], usage: &str) -> Result<String, String> {
    if args.len() != 1 {
        return Err(usage.to_owned());
    }
    Ok(args[0].clone())
}

pub fn positional(args: &[String], index: usize, usage: &str) -> Result<String, String> {
    args.iter()
        .filter(|arg| !arg.starts_with("--"))
        .nth(index)
        .cloned()
        .ok_or_else(|| usage.to_owned())
}

pub fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

pub fn required_record(args: &[String], usage: &str) -> Result<Value, String> {
    match flag_value(args, "--record") {
        Some(raw) => parse_json_value(&raw),
        None => Err(usage.to_owned()),
    }
}

/// Parses inline JSON or `@path` file JSON.
pub fn parse_json_value(raw: &str) -> Result<Value, String> {
    if let Some(path) = raw.strip_prefix('@') {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("could not read {path}: {error}"))?;
        serde_json::from_str(&text).map_err(|error| format!("{path} is not valid JSON: {error}"))
    } else {
        serde_json::from_str(raw).map_err(|error| format!("invalid JSON: {error}"))
    }
}

/// Parses `k=v` pairs; each value is JSON-decoded with a string fallback so
/// `port=8080` becomes a number while `url=http://x` stays a string.
pub fn kv_object(pairs: &[String]) -> Result<Value, String> {
    kv_map(pairs).map(|map| Value::Object(map.into_iter().collect()))
}

/// `k=v` pairs as a map for typed params taking `BTreeMap<String, Value>`.
pub fn kv_map(pairs: &[String]) -> Result<BTreeMap<String, Value>, String> {
    let mut object = BTreeMap::new();
    for pair in pairs {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| format!("expected k=v, got `{pair}`"))?;
        if key.trim().is_empty() {
            return Err(format!("expected k=v, got `{pair}`"));
        }
        let value: Value =
            serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_owned()));
        object.insert(key.to_owned(), value);
    }
    Ok(object)
}
