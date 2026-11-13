use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Strings are stable protocol capability identifiers, never role names.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum Capability {
    #[serde(rename = "live.read")]
    LiveRead,
    #[serde(rename = "points.read")]
    PointsRead,
    #[serde(rename = "points.adjust")]
    PointsAdjust,
    #[serde(rename = "rankings.read")]
    RankingsRead,
    #[serde(rename = "rankings.manage")]
    RankingsManage,
    #[serde(rename = "competitions.read")]
    CompetitionsRead,
    #[serde(rename = "competitions.manage")]
    CompetitionsManage,
    #[serde(rename = "workflows.read")]
    WorkflowsRead,
    #[serde(rename = "workflows.edit")]
    WorkflowsEdit,
    #[serde(rename = "workflows.toggle")]
    WorkflowsToggle,
    #[serde(rename = "automation.read")]
    AutomationRead,
    #[serde(rename = "automation.edit")]
    AutomationEdit,
    #[serde(rename = "automation.toggle")]
    AutomationToggle,
    #[serde(rename = "plugins.read")]
    PluginsRead,
    #[serde(rename = "plugins.toggle")]
    PluginsToggle,
    #[serde(rename = "plugins.settings.read")]
    PluginSettingsRead,
    #[serde(rename = "plugins.settings.write")]
    PluginSettingsWrite,
    #[serde(rename = "plugins.action.execute")]
    PluginActionExecute,
    #[serde(rename = "widgets.read")]
    WidgetsRead,
    #[serde(rename = "widgets.configure")]
    WidgetsConfigure,
    #[serde(rename = "commands.read")]
    CommandsRead,
    #[serde(rename = "http.request")]
    HttpRequest,
    #[serde(rename = "audio.play")]
    AudioPlay,
    #[serde(rename = "events.emit")]
    EventsEmit,
    #[serde(rename = "scripts.execute")]
    ScriptsExecute,
}

/// Explicit policy for future reusable delegated automation execution. This
/// is not itself an execution permit: the host must validate the saved closure
/// and current subjects before it may be used at runtime.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum DelegatedAutomationEffect {
    Log,
    Delay,
    Points,
    Http,
    Audio,
    Emit,
    Script,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutomationDelegation {
    pub effects: BTreeSet<DelegatedAutomationEffect>,
    pub allow_dynamic_targets: bool,
    /// Explicitly allow scoped changes to existing Owner definitions.
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_owner_edits: bool,
    /// Explicitly share Owner source, which can include sensitive values.
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_owner_source: bool,
    /// Allow scoped definitions authored by another moderator to be replaced.
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_other_author_edits: bool,
    /// Share full source of scoped definitions authored by other moderators.
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_other_author_source: bool,
}
fn is_false(value: &bool) -> bool {
    !value
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceKind {
    Live,
    Viewer,
    Ranking,
    Competition,
    Workflow,
    Automation,
    Plugin,
    Widget,
}

/// The two behavior tables have independent IDs and revisions. Never treat a
/// raw ID in one family as authority over the other family.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum AutomationFamily {
    Action,
    Event,
}
impl AutomationFamily {
    pub const fn scope(self) -> &'static str {
        match self {
            Self::Action => "action",
            Self::Event => "event",
        }
    }
    pub fn from_scope(scope: &str) -> Option<Self> {
        match scope {
            "action" => Some(Self::Action),
            "event" => Some(Self::Event),
            _ => None,
        }
    }
    pub fn grant_id(self, id: &str) -> String {
        format!("{}:{id}", self.scope())
    }
}

/// Closed remote projection. Payload JSON, scripts, plugin configuration and
/// dependency IDs are deliberately absent, rather than heuristically scrubbed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutomationSummary {
    pub kind: AutomationFamily,
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub definition_redacted: bool,
}

/// Persisted resource version, shared with metadata-aware domain adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceVersion {
    pub kind: ResourceKind,
    pub id: String,
    pub scope: String,
    pub revision: String,
}

/// A business result and the versions read/committed in the same transaction.
/// API adapters must put versions in the envelope, preserving domain shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Versioned<T> {
    pub value: T,
    pub resource_versions: Vec<ResourceVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<crate::CommandReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_instance_id: Option<String>,
}
impl<T> Versioned<T> {
    pub fn new(value: T, resource_versions: Vec<ResourceVersion>) -> Self {
        Self {
            value,
            resource_versions,
            command: None,
            host_instance_id: None,
        }
    }
    pub fn map<R>(self, map: impl FnOnce(T) -> R) -> Versioned<R> {
        Versioned {
            value: map(self.value),
            resource_versions: self.resource_versions,
            command: self.command,
            host_instance_id: self.host_instance_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Grant {
    #[serde(default)]
    pub capabilities: BTreeSet<Capability>,
    /// Missing kind means unrestricted within a capability. An empty set
    /// explicitly denies that resource kind; IDs never contain wildcards.
    #[serde(default)]
    pub resources: BTreeMap<ResourceKind, BTreeSet<String>>,
    #[serde(default)]
    pub settings_fields: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    pub actions: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    pub max_points_delta: Option<f64>,
    /// External effects deny by default. Unlike legacy resource restrictions,
    /// a missing or empty policy never means unrestricted access.
    #[serde(default, skip_serializing_if = "crate::EffectGrants::is_empty")]
    pub effects: crate::EffectGrants,
    /// Missing on legacy grants; reusable execution is bounded by this policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation_delegation: Option<AutomationDelegation>,
}

impl Grant {
    pub fn viewer() -> Self {
        Self {
            capabilities: [Capability::LiveRead].into(),
            ..Self::default()
        }
    }

    pub fn allows_resource(&self, kind: ResourceKind, id: &str) -> bool {
        if kind == ResourceKind::Automation && !valid_automation_grant_id(id) {
            return false;
        }
        self.resources.get(&kind).is_none_or(|ids| ids.contains(id))
    }

    pub fn allows_automation(&self, family: AutomationFamily, id: &str) -> bool {
        canonical_automation_id(id)
            && self.allows_resource(ResourceKind::Automation, &family.grant_id(id))
    }

    /// Scope-aware visibility for metadata, replay and original command results.
    pub fn allows_scoped_resource(&self, kind: ResourceKind, scope: &str, id: &str) -> bool {
        if kind == ResourceKind::Automation {
            AutomationFamily::from_scope(scope)
                .is_some_and(|family| self.allows_automation(family, id))
        } else {
            scope == "host" && self.allows_resource(kind, id)
        }
    }

    pub fn validate(&self) -> Result<(), crate::Error> {
        self.effects.validate()?;
        if let Some(policy) = &self.automation_delegation {
            if policy.effects.is_empty()
                || (policy.allow_owner_source && !policy.allow_owner_edits)
                || (policy.allow_other_author_source && !policy.allow_other_author_edits)
                || ![
                    Capability::AutomationRead,
                    Capability::AutomationEdit,
                    Capability::AutomationToggle,
                ]
                .iter()
                .all(|capability| self.capabilities.contains(capability))
                || policy.effects.iter().any(|effect| {
                    let required = match effect {
                        DelegatedAutomationEffect::Points => Some(Capability::PointsAdjust),
                        DelegatedAutomationEffect::Http => Some(Capability::HttpRequest),
                        DelegatedAutomationEffect::Audio => Some(Capability::AudioPlay),
                        DelegatedAutomationEffect::Emit => Some(Capability::EventsEmit),
                        DelegatedAutomationEffect::Script => Some(Capability::ScriptsExecute),
                        DelegatedAutomationEffect::Log | DelegatedAutomationEffect::Delay => None,
                    };
                    required.is_some_and(|capability| !self.capabilities.contains(&capability))
                })
            {
                return Err(crate::Error::Invalid(
                    "invalid automation delegation policy",
                ));
            }
        }
        if self.resources.len() > 32
            || self.settings_fields.len() > 256
            || self.actions.len() > 256
            || self
                .max_points_delta
                .is_some_and(|n| !n.is_finite() || n <= 0.0)
        {
            return Err(crate::Error::Invalid("invalid grant limits"));
        }
        for ids in self.resources.values() {
            if ids.len() > 256 || ids.iter().any(|id| !valid_id(id)) {
                return Err(crate::Error::Invalid("invalid grant resource restriction"));
            }
        }
        if self
            .resources
            .get(&ResourceKind::Automation)
            .is_some_and(|ids| ids.iter().any(|id| !valid_automation_grant_id(id)))
        {
            return Err(crate::Error::Invalid(
                "automation grants require action:<id> or event:<id>",
            ));
        }
        for (kind, ids) in self.settings_fields.iter().chain(self.actions.iter()) {
            if !valid_id(kind) || ids.len() > 256 || ids.iter().any(|id| !valid_id(id)) {
                return Err(crate::Error::Invalid("invalid grant resource restriction"));
            }
        }
        Ok(())
    }
}

fn canonical_automation_id(id: &str) -> bool {
    valid_id(id) && id.len() <= 128 && id.trim() == id
}
fn valid_automation_grant_id(id: &str) -> bool {
    id.split_once(':').is_some_and(|(family, id)| {
        AutomationFamily::from_scope(family).is_some() && canonical_automation_id(id)
    })
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.contains('*')
        && !value.chars().any(char::is_control)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ClientKind {
    Desktop,
    Browser,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteLimits {
    pub connections: u32,
    pub connections_per_principal: u32,
    pub frame_bytes: usize,
    pub params_bytes: usize,
    pub pending_calls: u32,
    pub requests_per_second: u32,
    pub request_burst: u32,
    #[serde(default = "default_command_limit")]
    pub command_ids_per_principal: u32,
}
fn default_command_limit() -> u32 {
    100_000
}
impl Default for RemoteLimits {
    fn default() -> Self {
        Self {
            connections: 8,
            connections_per_principal: 4,
            frame_bytes: 1024 * 1024,
            params_bytes: 256 * 1024,
            pending_calls: 32,
            requests_per_second: 30,
            request_burst: 60,
            command_ids_per_principal: default_command_limit(),
        }
    }
}
impl RemoteLimits {
    pub fn validate(&self) -> Result<(), crate::Error> {
        if !(1..=8).contains(&self.connections)
            || !(1..=4).contains(&self.connections_per_principal)
            || self.connections_per_principal > self.connections
            || !(4096..=1024 * 1024).contains(&self.frame_bytes)
            || !(1024..=256 * 1024).contains(&self.params_bytes)
            || self.params_bytes > self.frame_bytes
            || !(1..=32).contains(&self.pending_calls)
            || !(1..=30).contains(&self.requests_per_second)
            || self.request_burst < self.requests_per_second
            || self.request_burst > 60
            || !(1..=100_000).contains(&self.command_ids_per_principal)
        {
            return Err(crate::Error::Invalid(
                "invalid collaboration transport limits",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Configuration {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer: Option<PeerConfiguration>,
    #[serde(default = "default_bind")]
    pub bind_address: String,
    #[serde(default)]
    pub advertised_origin: Option<String>,
    #[serde(default)]
    pub allowed_origins: BTreeSet<String>,
    #[serde(default)]
    pub tls_certificate: Option<String>,
    #[serde(default)]
    pub tls_private_key: Option<String>,
    #[serde(default)]
    pub tls_proxy: bool,
    #[serde(default)]
    pub limits: RemoteLimits,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            enabled: false,
            peer: None,
            bind_address: default_bind(),
            advertised_origin: None,
            allowed_origins: BTreeSet::new(),
            tls_certificate: None,
            tls_private_key: None,
            tls_proxy: false,
            limits: RemoteLimits::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeerConfiguration {
    #[serde(default)]
    pub relay_urls: Vec<String>,
}

fn default_bind() -> String {
    "127.0.0.1:8788".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub host_id: String,
    pub host_instance_id: String,
    pub configuration: Configuration,
    pub configuration_revision: String,
    pub active_sessions: u32,
    pub pending_requests: u32,
    pub gateway: GatewayStatus,
    #[serde(default)]
    pub peer_gateway: PeerGatewayStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PeerGatewayStatus {
    #[serde(flatten)]
    pub gateway: GatewayStatus,
    pub endpoint_id: Option<String>,
    pub addresses: Vec<String>,
    pub relay_urls: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GatewayStatus {
    /// Internal listener incarnation; never a wire field or credential.
    #[serde(skip)]
    pub generation: String,
    pub available: bool,
    pub running: bool,
    pub connections: u32,
    pub configuration_revision: Option<String>,
    pub address: Option<String>,
    pub last_error: Option<String>,
    #[serde(default)]
    pub certificate_fingerprint: Option<String>,
    #[serde(default)]
    pub session_connections: BTreeMap<String, u32>,
}

/// Optional owner configuration CAS, kept separate from persisted settings.
/// Legacy callers can keep sending the original bare Configuration object.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureInput {
    #[serde(flatten)]
    pub configuration: Configuration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<String>,
}
impl<'de> Deserialize<'de> for ConfigureInput {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let mut value = serde_json::Value::deserialize(d)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| serde::de::Error::custom("configuration must be an object"))?;
        let expected_revision = object
            .remove("expectedRevision")
            .map(serde_json::from_value::<Option<String>>)
            .transpose()
            .map_err(serde::de::Error::custom)?
            .flatten();
        let configuration = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(Self {
            configuration,
            expected_revision,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InviteCreate {
    #[serde(default = "Grant::viewer")]
    pub grant: Grant,
}

/// Never implements Debug: the secret is returned only once to the owner.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InviteSecret {
    pub invite_id: String,
    pub secret: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Invitation {
    pub invite_id: String,
    pub grant: Grant,
    pub expires_at: i64,
    pub state: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Redeem {
    pub invite_id: String,
    pub secret: String,
    pub client_label: String,
    pub client_kind: ClientKind,
    pub exchange_secret_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PairingRequest {
    pub request_id: String,
    pub client_label: String,
    pub client_kind: ClientKind,
    pub grant: Grant,
    pub state: String,
    pub expires_at: i64,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Collect {
    pub request_id: String,
    pub exchange_secret: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub principal_id: String,
    pub session_id: String,
    pub access_token: String,
    pub access_expires_at: i64,
    pub refresh_token: Option<String>,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Principal {
    pub principal_id: String,
    pub client_label: String,
    pub client_kind: ClientKind,
    pub grant: Grant,
    pub grant_revision: String,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub session_id: String,
    pub principal_id: String,
    #[serde(default)]
    pub client_label: String,
    pub client_kind: ClientKind,
    pub created_at: i64,
    pub last_activity_at: i64,
    pub expires_at: i64,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Authenticated {
    pub principal_id: String,
    pub session_id: String,
    pub grant: Grant,
    pub grant_revision: String,
    pub client_kind: ClientKind,
    pub access_expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdParams {
    pub id: String,
}

/// Keyset pagination over immutable IDs, not a snapshot. Restart without a
/// cursor to discover records created before an already-consumed cursor.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListQuery {
    #[serde(default)]
    pub after: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

impl ListQuery {
    pub(crate) fn bounds(&self) -> Result<(&str, usize), crate::Error> {
        let limit = self.limit.unwrap_or(50);
        let after = self.after.as_deref().unwrap_or("");
        if !(1..=100).contains(&limit)
            || self.after.as_ref().is_some_and(|id| {
                uuid::Uuid::parse_str(id).map_or(true, |parsed| parsed.to_string() != *id)
            })
        {
            return Err(crate::Error::Invalid(
                "invalid collaboration list cursor or limit",
            ));
        }
        Ok((after, limit))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPage<T> {
    pub items: Vec<T>,
    pub next_after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GrantSet {
    pub principal_id: String,
    pub expected_revision: String,
    pub grant: Grant,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditQuery {
    #[serde(default)]
    pub after: i64,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntry {
    pub cursor: i64,
    pub timestamp: i64,
    pub principal_id: String,
    pub action: String,
    pub resource_id: String,
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<crate::AuditRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_limits_can_be_lowered_but_not_raise_the_host_safety_bounds() {
        let mut limits = RemoteLimits {
            connections: 2,
            connections_per_principal: 1,
            frame_bytes: 8192,
            params_bytes: 4096,
            pending_calls: 2,
            requests_per_second: 5,
            request_burst: 10,
            command_ids_per_principal: 5,
        };
        limits.validate().unwrap();
        limits.connections = 9;
        assert!(limits.validate().is_err());
        limits.connections = 2;
        limits.connections_per_principal = 3;
        assert!(limits.validate().is_err());
        limits.connections_per_principal = 1;
        limits.frame_bytes = 2 * 1024 * 1024;
        assert!(limits.validate().is_err());
        limits.frame_bytes = 8192;
        limits.params_bytes = 9000;
        assert!(limits.validate().is_err());
        limits.params_bytes = 4096;
        limits.pending_calls = 33;
        assert!(limits.validate().is_err());
        limits.pending_calls = 2;
        limits.request_burst = 1;
        assert!(limits.validate().is_err());
        limits.request_burst = 10;
        limits.command_ids_per_principal = 0;
        assert!(limits.validate().is_err());
        limits.command_ids_per_principal = 100_001;
        assert!(limits.validate().is_err());
    }

    #[test]
    fn grants_reject_unknown_authority_and_preserve_resource_restrictions() {
        let grant: Grant = serde_json::from_value(serde_json::json!({
            "capabilities":["workflows.edit"],"resources":{"workflow":["welcome"]}
        }))
        .unwrap();
        grant.validate().unwrap();
        assert!(grant.allows_resource(ResourceKind::Workflow, "welcome"));
        assert!(!grant.allows_resource(ResourceKind::Workflow, "another"));
        assert!(
            serde_json::from_value::<Grant>(serde_json::json!({"capabilities":["*"]})).is_err()
        );
        assert!(serde_json::from_value::<Grant>(
            serde_json::json!({"resources":{"worklfow":["welcome"]}})
        )
        .is_err());
        assert!(serde_json::from_value::<Grant>(serde_json::json!({"role":"owner"})).is_err());
        let wildcard: Grant =
            serde_json::from_value(serde_json::json!({"resources":{"workflow":["*"]}})).unwrap();
        assert!(wildcard.validate().is_err());
    }
    #[test]
    fn reusable_automation_policy_is_explicit_closed_and_requires_capabilities() {
        let old: Grant = serde_json::from_value(serde_json::json!({"capabilities":["automation.read","automation.edit","automation.toggle"]})).unwrap();
        assert!(old.automation_delegation.is_none());
        assert!(serde_json::to_value(&old)
            .unwrap()
            .get("automationDelegation")
            .is_none());
        let mut grant = old;
        grant.automation_delegation = Some(AutomationDelegation {
            effects: [
                DelegatedAutomationEffect::Log,
                DelegatedAutomationEffect::Delay,
            ]
            .into(),
            allow_dynamic_targets: false,
            allow_owner_edits: false,
            allow_owner_source: false,
            allow_other_author_edits: false,
            allow_other_author_source: false,
        });
        grant.validate().unwrap();
        let serialized = serde_json::to_value(&grant).unwrap();
        let restored: Grant = serde_json::from_value(serialized.clone()).unwrap();
        assert_eq!(restored, grant);
        assert!(serialized["automationDelegation"]
            .get("allowOwnerEdits")
            .is_none());
        let mut shared = grant.clone();
        shared
            .automation_delegation
            .as_mut()
            .unwrap()
            .allow_owner_edits = true;
        let encoded = serde_json::to_value(&shared).unwrap();
        assert_eq!(encoded["automationDelegation"]["allowOwnerEdits"], true);
        assert_eq!(serde_json::from_value::<Grant>(encoded).unwrap(), shared);
        let mut source_only = grant.clone();
        source_only
            .automation_delegation
            .as_mut()
            .unwrap()
            .allow_owner_source = true;
        assert!(source_only.validate().is_err());
        shared
            .automation_delegation
            .as_mut()
            .unwrap()
            .allow_owner_source = true;
        assert!(shared.validate().is_ok());
        let mut extra = serialized.clone();
        extra["automationDelegation"]["effects"] = serde_json::json!(["plugin"]);
        assert!(serde_json::from_value::<Grant>(extra).is_err());
        let mut widened = serialized;
        widened["automationDelegation"]["effects"] = serde_json::json!(["http"]);
        assert!(serde_json::from_value::<Grant>(widened)
            .unwrap()
            .validate()
            .is_err());
        grant
            .automation_delegation
            .as_mut()
            .unwrap()
            .effects
            .clear();
        assert!(grant.validate().is_err());
        grant
            .automation_delegation
            .as_mut()
            .unwrap()
            .effects
            .insert(DelegatedAutomationEffect::Log);
        grant.capabilities.remove(&Capability::AutomationToggle);
        assert!(grant.validate().is_err());
    }
    #[test]
    fn automation_grants_bind_family_and_never_promote_ambiguous_legacy_ids() {
        let grant = Grant {
            resources: [(
                ResourceKind::Automation,
                ["event:same".into(), "action:with:colon".into()].into(),
            )]
            .into(),
            ..Grant::default()
        };
        grant.validate().unwrap();
        assert!(grant.allows_automation(AutomationFamily::Event, "same"));
        assert!(!grant.allows_automation(AutomationFamily::Action, "same"));
        assert!(grant.allows_automation(AutomationFamily::Action, "with:colon"));
        assert!(!grant.allows_scoped_resource(ResourceKind::Automation, "host", "same"));
        assert!(!grant.allows_scoped_resource(ResourceKind::Automation, "action", "same"));
        assert!(grant.allows_scoped_resource(ResourceKind::Automation, "event", "same"));
        for bad in [
            "same",
            "Event:same",
            "host:same",
            "event:",
            "event: same",
            "action:same ",
            "event:*",
            "event:control\n",
        ] {
            let invalid = Grant {
                resources: [(ResourceKind::Automation, [bad.into()].into())].into(),
                ..Grant::default()
            };
            assert!(invalid.validate().is_err(), "{bad}");
        }
        let legacy = Grant {
            resources: [(ResourceKind::Automation, ["same".into()].into())].into(),
            ..Grant::default()
        };
        assert!(!legacy.allows_automation(AutomationFamily::Event, "same"));
        let empty = Grant {
            resources: [(ResourceKind::Automation, Default::default())].into(),
            ..Grant::default()
        };
        assert!(!empty.allows_automation(AutomationFamily::Event, "same"));
        assert!(Grant::default().allows_automation(AutomationFamily::Action, "same"));
    }
}
