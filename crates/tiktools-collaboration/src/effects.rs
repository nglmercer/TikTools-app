//! Closed external-effect grants. A single rule must cover the complete
//! request; combining destinations, methods or budgets from separate rules
//! would silently widen an Owner's grant.
use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{Capability, Error, Grant};

pub const MAX_EFFECT_BODY_BYTES: usize = 256 * 1024;
pub const MAX_EFFECT_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_EFFECT_GRANT_BYTES: usize = 256 * 1024;
pub const PUBLIC_EFFECT_RENDER_FIELDS: &[&str] = &[
    "event.id",
    "event.type",
    "event.timestamp",
    "event.user.id",
    "event.user.uniqueId",
    "event.user.nickname",
    "event.creator.uniqueId",
    "event.creator.roomId",
    "event.data.comment",
    "event.data.count",
    "event.data.likeCount",
    "event.data.totalLikeCount",
    "event.data.giftId",
    "event.data.giftName",
    "event.data.diamondCount",
    "event.data.repeatCount",
    "event.data.repeatEnd",
    "event.data.giftType",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectGrants {
    #[serde(default)]
    pub http: Vec<HttpEffectGrant>,
    #[serde(default)]
    pub audio: Vec<AudioEffectGrant>,
    #[serde(default)]
    pub emit_topics: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HttpEffectGrant {
    pub origin: String,
    pub methods: BTreeSet<String>,
    pub paths: BTreeSet<String>,
    pub query_fields: BTreeSet<String>,
    pub header_names: BTreeSet<String>,
    pub render_fields: BTreeSet<String>,
    pub body_fields: BTreeSet<String>,
    pub allow_text_body: bool,
    pub max_body_bytes: usize,
    pub max_response_bytes: usize,
    pub allow_private_network: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioEffectGrant {
    pub file_refs: BTreeSet<String>,
    pub output: String,
    pub max_volume: f64,
    pub overlaps: BTreeSet<String>,
}

/// Already rendered request data. Callers retain the exact source/approval
/// fence and current subject admission around final dispatch separately.
#[derive(Debug)]
pub struct HttpEffectRequest<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub header_names: &'a [String],
    pub render_fields: &'a [String],
    pub body: Option<&'a str>,
    pub response_limit: usize,
    pub allow_private_network: bool,
}

impl EffectGrants {
    pub fn is_empty(&self) -> bool {
        self.http.is_empty() && self.audio.is_empty() && self.emit_topics.is_empty()
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.http.len() > 32
            || self.audio.len() > 32
            || self.emit_topics.len() > 256
            || self.emit_topics.iter().any(|s| !valid_emit_topic(s))
        {
            return Err(Error::Invalid("invalid effect grant limits or topics"));
        }
        for rule in &self.http {
            rule.validate()?;
        }
        for rule in &self.audio {
            rule.validate()?;
        }
        if serde_json::to_vec(self)
            .map_err(|_| Error::Invalid("invalid effect grant"))?
            .len()
            > MAX_EFFECT_GRANT_BYTES
        {
            return Err(Error::Invalid("effect grant exceeds its byte budget"));
        }
        Ok(())
    }
}

impl HttpEffectGrant {
    pub fn validate(&self) -> Result<(), Error> {
        if !valid_origin(&self.origin, self.allow_private_network)
            || self.methods.is_empty()
            || self.methods.len() > 7
            || self.methods.iter().any(|s| !valid_method(s))
            || self.paths.is_empty()
            || self.paths.len() > 64
            || self.paths.iter().any(|s| !valid_effect_path(s))
            || self.query_fields.len() > 64
            || self.query_fields.iter().any(|s| !valid_field(s))
            || self.header_names.len() > 64
            || self.header_names.iter().any(|s| !valid_header(s))
            || self.render_fields.len() > PUBLIC_EFFECT_RENDER_FIELDS.len()
            || self
                .render_fields
                .iter()
                .any(|s| !PUBLIC_EFFECT_RENDER_FIELDS.contains(&s.as_str()))
            || self.body_fields.len() > 256
            || self.body_fields.iter().any(|s| !valid_field(s))
            || self.max_body_bytes > MAX_EFFECT_BODY_BYTES
            || self.max_response_bytes == 0
            || self.max_response_bytes > MAX_EFFECT_RESPONSE_BYTES
        {
            return Err(Error::Invalid("invalid HTTP effect restriction"));
        }
        Ok(())
    }

    fn allows(&self, request: &HttpEffectRequest<'_>, url: &Url) -> bool {
        self.origin == url.origin().ascii_serialization()
            && self.methods.contains(request.method)
            && self.paths.contains(url.path())
            && (!request.allow_private_network || self.allow_private_network)
            && (!local_host(url) || self.allow_private_network)
            && request.response_limit > 0
            && request.response_limit <= self.max_response_bytes
            && request.header_names.len() <= 64
            && request
                .header_names
                .iter()
                .map(|name| name.to_ascii_lowercase())
                .collect::<BTreeSet<_>>()
                .len()
                == request.header_names.len()
            && request.header_names.iter().all(|name| {
                valid_header(&name.to_ascii_lowercase())
                    && self.header_names.contains(&name.to_ascii_lowercase())
            })
            && request
                .render_fields
                .iter()
                .all(|field| self.render_fields.contains(field))
            && self.allows_query(url)
            && self.allows_body(request.body)
    }

    fn allows_query(&self, url: &Url) -> bool {
        let mut keys = BTreeSet::new();
        url.query_pairs().all(|(key, _)| {
            valid_field(&key)
                && self.query_fields.contains(key.as_ref())
                && keys.insert(key.into_owned())
        })
    }

    fn allows_body(&self, body: Option<&str>) -> bool {
        let Some(body) = body else {
            return true;
        };
        if body.len() > self.max_body_bytes {
            return false;
        }
        if body.trim().is_empty() {
            return true;
        }
        match serde_json::from_str::<serde_json::Value>(body) {
            Ok(serde_json::Value::Object(fields)) => {
                fields.keys().all(|key| self.body_fields.contains(key))
            }
            // A JSON scalar/array is not a top-level field-restricted object.
            Ok(_) | Err(_) => self.allow_text_body,
        }
    }
}

impl AudioEffectGrant {
    pub fn validate(&self) -> Result<(), Error> {
        if self.file_refs.is_empty()
            || self.file_refs.len() > 256
            || self.file_refs.iter().any(|s| !valid_file_ref(s))
            || self.output != "host-default"
            || !self.max_volume.is_finite()
            || !(0.0..=1.0).contains(&self.max_volume)
            || self.overlaps.is_empty()
            || self.overlaps.len() > 3
            || self
                .overlaps
                .iter()
                .any(|s| !matches!(s.as_str(), "allow" | "restart" | "drop"))
        {
            return Err(Error::Invalid("invalid audio effect restriction"));
        }
        Ok(())
    }
}

impl Grant {
    pub fn allows_http_effect(&self, request: &HttpEffectRequest<'_>) -> bool {
        if !self.capabilities.contains(&Capability::HttpRequest)
            || self.effects.validate().is_err()
            || !request.render_fields.is_empty()
                && !self.capabilities.contains(&Capability::LiveRead)
            || request.url.len() > 8192
            || !valid_method(request.method)
        {
            return false;
        }
        let Ok(url) = Url::parse(request.url) else {
            return false;
        };
        if !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || !valid_effect_path(url.path())
            || url.as_str() != request.url
        {
            return false;
        }
        self.effects
            .http
            .iter()
            .any(|rule| rule.validate().is_ok() && rule.allows(request, &url))
    }

    /// file_ref must be the host-resolved canonical path, not caller metadata.
    pub fn allows_audio_effect(
        &self,
        file_ref: &str,
        output: &str,
        volume: f64,
        overlap: &str,
    ) -> bool {
        self.capabilities.contains(&Capability::AudioPlay)
            && self.effects.validate().is_ok()
            && volume.is_finite()
            && (0.0..=1.0).contains(&volume)
            && self.effects.audio.iter().any(|rule| {
                rule.validate().is_ok()
                    && rule.file_refs.contains(file_ref)
                    && rule.output == output
                    && volume <= rule.max_volume
                    && rule.overlaps.contains(overlap)
            })
    }

    pub fn allows_emit_topic(&self, topic: &str) -> bool {
        self.capabilities.contains(&Capability::EventsEmit)
            && self.effects.validate().is_ok()
            && valid_emit_topic(topic)
            && self.effects.emit_topics.contains(topic)
    }
}

fn valid_method(method: &str) -> bool {
    matches!(
        method,
        "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
    )
}

fn valid_origin(origin: &str, allow_private: bool) -> bool {
    let Ok(url) = Url::parse(origin) else {
        return false;
    };
    url.origin().ascii_serialization() == origin
        && url.path() == "/"
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && (url.scheme() == "https" || url.scheme() == "http" && allow_private && local_host(&url))
}

fn local_host(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(host)) => host == "localhost" || host.ends_with(".localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        Some(url::Host::Ipv6(ip)) => {
            ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local()
        }
        None => false,
    }
}

pub fn valid_effect_path(path: &str) -> bool {
    if !path.starts_with('/')
        || path.starts_with("//")
        || path.len() > 2048
        || path.bytes().any(|c| {
            !(0x21..=0x7e).contains(&c)
                || matches!(c, b'%' | b'\\' | b'?' | b'#' | b'*' | b'{' | b'}')
        })
    {
        return false;
    }
    Url::parse(&format!("https://grant.invalid{path}")).is_ok_and(|url| {
        url.host_str() == Some("grant.invalid")
            && url.path() == path
            && url.query().is_none()
            && url.fragment().is_none()
    })
}

fn valid_field(field: &str) -> bool {
    !field.is_empty()
        && field.len() <= 256
        && field.trim() == field
        && !field.contains('*')
        && !field.chars().any(char::is_control)
}

fn valid_header(header: &str) -> bool {
    !matches!(
        header,
        "host"
            | "content-length"
            | "transfer-encoding"
            | "connection"
            | "upgrade"
            | "proxy-authorization"
            | "proxy-connection"
            | "te"
            | "trailer"
    ) && !header.is_empty()
        && header.len() <= 128
        && header.bytes().all(|c| {
            c.is_ascii_lowercase()
                || c.is_ascii_digit()
                || matches!(
                    c,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

fn valid_file_ref(file: &str) -> bool {
    !file.is_empty()
        && file.len() <= 4096
        && file.trim() == file
        && !file.chars().any(char::is_control)
        && !file.contains(['*', '{', '}'])
        && (file.starts_with('/')
            || file.starts_with("\\\\")
            || file.as_bytes().get(1) == Some(&b':')
                && file.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                && matches!(file.as_bytes().get(2), Some(b'/' | b'\\')))
        && !file
            .split(['/', '\\'])
            .any(|part| matches!(part, "." | ".."))
}

pub fn valid_emit_topic(topic: &str) -> bool {
    !topic.is_empty()
        && topic.len() <= 64
        && topic.bytes().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'.' | b'_' | b'-')
        })
}

#[cfg(test)]
mod tests;
