//! Structured command specifications: the single source for help output.
//!
//! Every domain module owns a [`DomainSpec`] whose usage strings are the same
//! constants its parser reports in errors, so help text cannot drift from
//! what the parser accepts. [`registry`] aggregates the domains; [`render`]
//! produces the human page and [`render_json`] the machine-readable form an
//! agent reads with `tiktools help --json` — both offline, without a host.

use serde_json::{json, Value};

use super::i18n::{self, Locale};

/// Catalog key of one verb summary: `verb.<domain>.<name>` with the spaces
/// of compound names collapsed to dots (`settings reset` →
/// `verb.plugin.settings.reset`).
pub fn verb_key(domain: &str, name: &str) -> String {
    format!("verb.{domain}.{}", name.replace(' ', "."))
}

/// Catalog key of one domain summary: `domain.<domain>`.
pub fn domain_key(domain: &str) -> String {
    format!("domain.{domain}")
}

/// Catalog key of one convention text: `convention.<topic>` with spaces as
/// dashes.
pub fn convention_key(topic: &str) -> String {
    format!("convention.{}", topic.replace(' ', "-"))
}

/// One callable form. `usage` is the full line (`plugin get <id>`); it is the
/// exact string the parser reports when the form is malformed.
pub struct VerbSpec {
    pub name: &'static str,
    pub usage: &'static str,
    pub summary: &'static str,
    /// True for forms that mutate or destroy state (delete, reset, stop...).
    /// Agents use this to decide whether to confirm before executing.
    pub destructive: bool,
}

impl VerbSpec {
    pub const fn new(name: &'static str, usage: &'static str, summary: &'static str) -> Self {
        Self {
            name,
            usage,
            summary,
            destructive: false,
        }
    }

    /// Marks a form that mutates or destroys state.
    pub const fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }
}

/// Derives the verb path from a usage line: the tokens between the domain
/// name and the first argument or flag. Tests assert it matches the explicit
/// `name` so neither can drift.
#[cfg(test)]
pub fn derive_name(usage: &str) -> String {
    usage
        .split_whitespace()
        .skip(1)
        .take_while(|token| {
            !token.starts_with('-')
                && token
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct DomainSpec {
    pub domain: &'static str,
    pub summary: &'static str,
    pub verbs: &'static [VerbSpec],
}

/// Global CLI behaviour that is not specific to one command. Rendered as the
/// footer of the human page and as the `conventions` object in JSON help.
const CONVENTIONS: &[(&str, &str)] = &[
    (
        "host resolution",
        "commands run against the running host over local IPC; when no host is running they fail with host_unavailable instead of starting a second runtime. Use `tiktools --json system info` as the lightweight probe: exit 0 means a host answered, exit 3 means the IPC host is unavailable. --standalone runs the command against an isolated in-process runtime instead.",
    ),
    (
        "output",
        "stdout carries the result; diagnostics go to stderr. --json prints the raw result object; without it, lists print one line per item. json values may use @path to read from a file. Human help follows the display locale: TIKTOOLS_LANG overrides the LC_ALL/LC_MESSAGES/LANG chain, and machine output stays locale-independent.",
    ),
    (
        "exit codes",
        "0 ok, 1 operation error, 2 usage error, 3 transport failure.",
    ),
    (
        "secrets",
        "live connect/pick read the session cookie from --session-cookie or the TIKTOOLS_SESSION_COOKIE environment variable (the flag wins); the value never appears in diagnostics. api call redacts secrets from its output unless --secrets-visible is passed, and refuses destructive methods without --confirm.",
    ),
];

/// Every domain in dispatch order. Feature-gated domains appear only when
/// the binary carries the feature, matching what the parser accepts.
pub fn registry() -> Vec<DomainSpec> {
    #[allow(unused_mut)]
    let mut domains = vec![
        super::plugins::SPEC,
        super::packages::REGISTRY_SPEC,
        super::processors::SPEC,
        super::live::SPEC,
        super::points::SPEC,
        super::rankings::SPEC,
        super::competitions::SPEC,
        super::automation::SPEC,
        super::globals::SPEC,
        super::workflows::SPEC,
        super::media::SPEC,
        super::system::SPEC,
        super::template::SPEC,
        super::api::SPEC,
        super::raw::SPEC,
    ];
    #[cfg(feature = "cloud-auth")]
    {
        domains.insert(1, super::cloud::SPEC);
        domains.insert(2, super::publishers::SPEC);
    }
    domains.push(super::web::SPEC);
    domains
}

fn translations_json(key: &str) -> Value {
    i18n::all_translations(key)
        .into_iter()
        .map(|(locale, text)| (locale.to_owned(), json!(text)))
        .collect::<serde_json::Map<String, Value>>()
        .into()
}

fn verb_json(domain: &str, verb: &VerbSpec) -> Value {
    json!({
        "name": verb.name,
        "usage": verb.usage,
        "summary": verb.summary,
        "summaries": translations_json(&verb_key(domain, verb.name)),
        "destructive": verb.destructive,
    })
}

/// Machine-readable help for agents: every command form with its usage line,
/// summary and destructive marker, plus the global CLI conventions.
/// Pass a filtered slice for one-domain output.
/// Version of the machine-readable help contract. Additive fields keep the
/// version; a breaking reshape bumps it so agents and documentation
/// generators can detect the change.
pub const JSON_SCHEMA: u32 = 1;

pub fn render_json(domains: &[&DomainSpec]) -> Value {
    json!({
        "schema": JSON_SCHEMA,
        "usage": "tiktools [--json] [--standalone] <command> [args]",
        "commands": domains.iter().map(|domain| json!({
            "domain": domain.domain,
            "summary": domain.summary,
            "summaries": translations_json(&domain_key(domain.domain)),
            "verbs": domain.verbs.iter().map(|verb| verb_json(domain.domain, verb)).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "conventions": CONVENTIONS.iter().map(|(topic, text)| json!({
            "topic": topic,
            "topics": translations_json(&format!("topic.{}", topic.replace(' ', "-"))),
            "text": text,
            "texts": translations_json(&convention_key(topic)),
        })).collect::<Vec<_>>(),
    })
}

fn render_domain(text: &mut String, domain: &DomainSpec, locale: Locale) {
    use std::fmt::Write;
    let summary = i18n::text(locale, &domain_key(domain.domain), domain.summary);
    let _ = writeln!(text, "\n{} — {}", domain.domain, summary);
    for verb in domain.verbs {
        let _ = writeln!(text, "  {}", verb.usage);
        if !verb.summary.is_empty() {
            let summary = i18n::text(locale, &verb_key(domain.domain, verb.name), verb.summary);
            let _ = writeln!(text, "      {}", summary);
        }
    }
}

/// Selects one domain by name or the whole registry. Shared by the human
/// and JSON renderers so `help plugin` and `help plugin --json` agree.
pub fn select<'a>(
    domains: &'a [DomainSpec],
    filter: Option<&str>,
    locale: Locale,
) -> Result<Vec<&'a DomainSpec>, String> {
    match filter {
        Some(name) => {
            let found: Vec<&DomainSpec> = domains
                .iter()
                .filter(|domain| domain.domain == name)
                .collect();
            if found.is_empty() {
                return Err(i18n::text(
                    locale,
                    "chrome.unknown-command",
                    "unknown command `{name}`",
                )
                .replace("{name}", name));
            }
            Ok(found)
        }
        None => Ok(domains.iter().collect()),
    }
}

/// Human help page in the resolved display locale. `filter` selects one
/// domain (`tiktools help plugin`); the full page renders when it is `None`.
/// Usage lines are parser syntax and stay in English in every locale.
pub fn render(
    domains: &[DomainSpec],
    filter: Option<&str>,
    locale: Locale,
) -> Result<String, String> {
    use std::fmt::Write;
    let selected = select(domains, filter, locale)?;
    let usage_label = i18n::text(locale, "chrome.usage-label", "usage");
    let mut text = format!("{usage_label}: tiktools [--json] [--standalone] <command> [args]\n");
    for domain in selected {
        render_domain(&mut text, domain, locale);
    }
    if filter.is_none() {
        let header = i18n::text(locale, "chrome.conventions", "conventions");
        let _ = writeln!(text, "\n{header}");
        for (topic, description) in CONVENTIONS {
            let label = i18n::text(locale, &format!("topic.{}", topic.replace(' ', "-")), topic);
            let description = i18n::text(locale, &convention_key(topic), description);
            let _ = writeln!(text, "  {label}: {description}");
        }
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Domains the dispatcher routes to. The registry must cover each one;
    /// a new domain added to `parse_command` without a spec fails here.
    const DISPATCHED: &[&str] = &[
        "plugin",
        "cloud",
        "publisher",
        "registry",
        "processor",
        "live",
        "points",
        "ranking",
        "competition",
        "automation",
        "globals",
        "workflow",
        "media",
        "system",
        "template",
        "api",
        "host",
        "web",
    ];

    #[test]
    fn registry_covers_every_dispatched_domain() {
        let domains = registry();
        for expected in DISPATCHED {
            assert!(
                domains.iter().any(|domain| domain.domain == *expected),
                "missing spec for dispatched domain `{expected}`"
            );
        }
    }

    #[test]
    fn usage_lines_start_with_their_domain_and_names_match_the_syntax() {
        for domain in registry() {
            assert!(!domain.verbs.is_empty(), "{} has no verbs", domain.domain);
            for verb in domain.verbs {
                let first = verb.usage.split_whitespace().next().unwrap_or("");
                assert!(
                    first == domain.domain
                        // web/server forms lead with the executable subcommand.
                        || domain.domain == "host" && first == "rpc"
                        || domain.domain == "web" && first == "web",
                    "`{}` does not start with its domain `{}`",
                    verb.usage,
                    domain.domain,
                );
                let derived = derive_name(verb.usage);
                // Flag-compound forms (`update --all`, `host --stdio`) extend
                // the derivation with the flags that make them distinct verbs.
                let matches = derived == verb.name
                    || verb
                        .name
                        .strip_prefix(&derived)
                        .is_some_and(|rest| rest.starts_with(" --"));
                if !derived.is_empty() {
                    assert!(
                        matches,
                        "explicit name `{}` disagrees with usage syntax in `{}`",
                        verb.name, verb.usage
                    );
                }
            }
        }
    }

    #[test]
    fn usage_lines_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for domain in registry() {
            for verb in domain.verbs {
                assert!(seen.insert(verb.usage), "duplicate usage `{}`", verb.usage);
            }
        }
    }

    #[test]
    fn json_help_is_machine_readable() {
        let domains = registry();
        let refs: Vec<&DomainSpec> = domains.iter().collect();
        let json = render_json(&refs);
        assert!(json["commands"].as_array().unwrap().len() >= DISPATCHED.len() - 2);
        assert!(json["conventions"].as_array().unwrap().len() >= 3);
        assert_eq!(json["schema"].as_u64(), Some(JSON_SCHEMA as u64));
        let first = &json["commands"][0]["verbs"][0];
        assert!(first.get("usage").is_some());
        assert!(first.get("destructive").is_some());
    }
}
