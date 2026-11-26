//! CLI help translations. English strings live inline in the command specs
//! as the fallback catalog; each supported locale carries a complete catalog
//! here, keyed mechanically from the spec identity:
//!
//! - `verb.<domain>.<name>` — verb summaries (`settings reset` → `verb.plugin.settings.reset`)
//! - `domain.<domain>` — domain summaries
//! - `convention.<topic>` — convention topics, spaces to dashes
//! - `chrome.<id>` — renderer boilerplate (usage label, hint, headers)
//!
//! Completeness is enforced by test: adding a command without its
//! translations fails the build, so catalogs cannot drift from the registry.
//! Usage lines are parser syntax and are never translated.

mod es;
mod fr;
mod ja;
mod pt;
mod zh;

/// Locales with a complete CLI help catalog. `en` is the inline fallback and
/// is not part of [`CATALOG_LOCALES`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Locale {
    En,
    Es,
    Pt,
    Fr,
    Zh,
    Ja,
}

pub const CATALOG_LOCALES: &[Locale] =
    &[Locale::Es, Locale::Pt, Locale::Fr, Locale::Zh, Locale::Ja];

impl Locale {
    pub fn code(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Es => "es",
            Locale::Pt => "pt",
            Locale::Fr => "fr",
            Locale::Zh => "zh",
            Locale::Ja => "ja",
        }
    }

    fn from_language(language: &str) -> Locale {
        match language {
            "es" => Locale::Es,
            "pt" => Locale::Pt,
            "fr" => Locale::Fr,
            "zh" => Locale::Zh,
            "ja" => Locale::Ja,
            _ => Locale::En,
        }
    }

    /// Display locale for human help: `TIKTOOLS_LANG` wins over the standard
    /// `LC_ALL`/`LC_MESSAGES`/`LANG` chain; regional variants resolve to
    /// their base language (`es_CR` → es). Machine output (`--json`) is
    /// locale-independent and carries every translation.
    pub fn resolve() -> Locale {
        let explicit = std::env::var("TIKTOOLS_LANG").ok();
        let ambient = explicit
            .filter(|value| !value.is_empty())
            .or_else(|| {
                std::env::var("LC_ALL")
                    .ok()
                    .filter(|value| !value.is_empty())
            })
            .or_else(|| {
                std::env::var("LC_MESSAGES")
                    .ok()
                    .filter(|value| !value.is_empty())
            })
            .or_else(|| std::env::var("LANG").ok().filter(|value| !value.is_empty()));
        let Some(value) = ambient else {
            return Locale::En;
        };
        let base = value
            .split(['-', '_', '.'])
            .next()
            .unwrap_or("")
            .to_lowercase();
        Locale::from_language(&base)
    }
}

fn catalog(locale: Locale, key: &str) -> Option<&'static str> {
    match locale {
        Locale::En => None,
        Locale::Es => es::lookup(key),
        Locale::Pt => pt::lookup(key),
        Locale::Fr => fr::lookup(key),
        Locale::Zh => zh::lookup(key),
        Locale::Ja => ja::lookup(key),
    }
}

/// Resolves one key for a locale, falling back to the inline English text
/// when the key is absent (English itself always falls back).
pub fn text<'a>(locale: Locale, key: &str, fallback: &'a str) -> &'a str {
    if locale == Locale::En {
        return fallback;
    }
    match catalog(locale, key) {
        // &'static coerces to 'a; no translation ever outlives the binary.
        Some(value) if !value.is_empty() => value,
        _ => fallback,
    }
}

/// Every translation of one key across catalog locales, for the JSON help
/// embedding. Locales without a translation are absent from the map.
pub fn all_translations(key: &str) -> Vec<(&'static str, &'static str)> {
    CATALOG_LOCALES
        .iter()
        .filter_map(|locale| catalog(*locale, key).map(|value| (locale.code(), value)))
        .collect()
}

/// Renderer boilerplate keys, translated like the spec summaries.
#[cfg(test)]
pub const CHROME_KEYS: &[&str] = &[
    "chrome.usage-label",
    "chrome.conventions",
    "chrome.hint",
    "chrome.unknown-command",
    "chrome.help-arity",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::spec::{convention_key, domain_key, registry, verb_key};

    /// Every registry entry and chrome string carries a translation in every
    /// catalog locale: adding a command without its translations fails here,
    /// so catalogs cannot drift from the registry. The failure lists exactly
    /// the keys to translate.
    #[test]
    fn catalogs_cover_the_whole_registry() {
        let mut keys: Vec<String> = CHROME_KEYS.iter().map(|key| key.to_string()).collect();
        for domain in registry() {
            keys.push(domain_key(domain.domain));
            for verb in domain.verbs {
                keys.push(verb_key(domain.domain, verb.name));
            }
        }
        for topic in ["host resolution", "output", "exit codes", "secrets"] {
            keys.push(convention_key(topic));
            keys.push(format!("topic.{}", topic.replace(' ', "-")));
        }
        let mut missing = Vec::new();
        for locale in CATALOG_LOCALES {
            for key in &keys {
                if catalog(*locale, key).is_none() {
                    missing.push(format!("{}: {key}", locale.code()));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "missing translations:
{}",
            missing.join(
                "
"
            )
        );
    }

    /// Catalogs must not carry orphan keys: a renamed or deleted command
    /// leaves its translation behind otherwise.
    #[test]
    fn catalogs_carry_no_orphan_keys() {
        // Orphans are detected indirectly: every catalog is a complete match
        // keyed from the registry, and the completeness test above proves
        // coverage. A hand-added orphan key would still compile, so scan the
        // sources for keys absent from the registry.
        let mut valid: std::collections::BTreeSet<String> =
            CHROME_KEYS.iter().map(|key| key.to_string()).collect();
        for domain in registry() {
            valid.insert(domain_key(domain.domain));
            for verb in domain.verbs {
                valid.insert(verb_key(domain.domain, verb.name));
            }
        }
        for topic in ["host resolution", "output", "exit codes", "secrets"] {
            valid.insert(convention_key(topic));
            valid.insert(format!("topic.{}", topic.replace(' ', "-")));
        }
        for file in ["es.rs", "pt.rs", "fr.rs", "zh.rs", "ja.rs"] {
            let source = std::fs::read_to_string(format!(
                "{}/src/commands/i18n/{file}",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap();
            for line in source.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix('"') {
                    if let Some(key) = rest.split('"').next() {
                        assert!(
                            valid.contains(key),
                            "{file}: orphan translation key `{key}`"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn locale_resolution_uses_base_language() {
        assert_eq!(Locale::from_language("es"), Locale::Es);
        assert_eq!(Locale::from_language("en"), Locale::En);
        assert_eq!(Locale::from_language("xx"), Locale::En);
    }
}
