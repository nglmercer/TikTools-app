//! Single authoritative registration, risk annotations and remote method policies.
use crate::{
    modules, policy, registry::MethodRegistry, ControlApi, ControlRouter, RemoteCapability,
};

impl ControlApi {
    /// Registers every domain module plus `rpc.*` discovery into `router`.
    /// This is the single source of truth for the control surface: `new`
    /// builds hosts from it and out-of-tree parity tests enumerate it.
    pub fn register_all(router: &mut ControlRouter) {
        modules::system::register(router);
        modules::collaboration::register(router);
        modules::plugins::register(router);
        #[cfg(feature = "plugin-install")]
        modules::packages::register(router);
        #[cfg(feature = "plugin-install")]
        modules::registry::register(router);
        #[cfg(feature = "cloud-auth")]
        modules::cloud::register(router);
        #[cfg(feature = "cloud-auth")]
        modules::sync::register(router);
        #[cfg(all(feature = "cloud-auth", feature = "http"))]
        modules::integrations::register(router);
        #[cfg(all(feature = "cloud-auth", feature = "http"))]
        modules::teams::register(router);
        #[cfg(all(feature = "cloud-auth", feature = "http"))]
        modules::bundles::register(router);
        #[cfg(all(feature = "cloud-auth", feature = "http", feature = "plugin-install"))]
        modules::publisher::register(router);
        modules::settings::register(router);
        modules::live::register(router);
        modules::points::register(router);
        modules::rankings::register(router);
        modules::competitions::register(router);
        modules::automation::register(router);
        modules::automation_adoption::register(router);
        modules::automation_approval::register(router);
        modules::automation_definition::register(router);
        modules::automation_effects::register(router);
        modules::automation_library::register(router);
        modules::processors::register(router);
        modules::media::register(router);
        modules::app::register(router);
        modules::updates::register(router);
        modules::globals::register(router);
        modules::creators::register(router);
        modules::analytics::register(router);
        modules::gifts::register(router);
        modules::ui::register(router);
        modules::widgets::register(router);
        modules::workflows::register(router);
        // Agent-facing risk metadata: destructive ops delete or reset
        // persisted state, so agents should confirm before calling them.
        for name in [
            "plugins.uninstall",
            "plugins.settings.reset",
            "automation.delete",
            "globals.delete",
            "globals.typed.replace",
            "sync.v2.variables.apply",
            "workflows.delete",
            "points.reset",
            "competitions.delete",
            "competitions.cancel",
            "rankings.delete",
            "rankings.reset",
            "creators.history.clear",
            "system.shutdown",
        ] {
            router.set_flags(name, true, false);
        }
        let registry = std::sync::Arc::new(std::sync::RwLock::new(MethodRegistry::default()));
        modules::rpc::register(router, std::sync::Arc::clone(&registry));
        // Side-effect metadata never grants authority. Each supported method
        // needs explicit policy; unlisted methods stay denied.
        for name in [
            "live.status",
            "live.monitor.status",
            "ui.feed.backfill",
            "ui.feed.browser",
            "ui.feed.creator",
        ] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::LiveRead),
            );
        }
        router.set_remote_policy(
            "workflows.disable",
            policy::RemotePolicy::Capability(RemoteCapability::WorkflowsToggle),
        );
        router.set_remote_policy(
            "workflows.delete",
            policy::RemotePolicy::Capability(RemoteCapability::WorkflowsEdit),
        );
        for name in ["automation.list", "automation.get", "automation.source.get"] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::AutomationRead),
            );
        }
        router.set_remote_policy(
            "competitions.summary",
            policy::RemotePolicy::Capability(RemoteCapability::CompetitionsRead),
        );
        for name in [
            "competitions.start",
            "competitions.resume",
            "competitions.pause",
            "competitions.cancel",
        ] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::CompetitionsManage),
            );
        }
        for name in ["automation.disable", "automation.enable"] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::AutomationToggle),
            );
        }
        router.set_remote_policy(
            "automation.delete",
            policy::RemotePolicy::Capability(RemoteCapability::AutomationEdit),
        );
        for name in ["automation.create", "automation.update"] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::AutomationEdit),
            );
        }

        for name in [
            "points.viewer.get",
            "points.leaderboard",
            "points.info",
            "points.view",
        ] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::PointsRead),
            );
        }
        router.set_remote_policy(
            "points.adjust",
            policy::RemotePolicy::Capability(RemoteCapability::PointsAdjust),
        );
        router.set_remote_policy(
            "collaboration.commands.get",
            policy::RemotePolicy::Capability(RemoteCapability::CommandsRead),
        );
        for name in [
            "rpc.discover",
            "rpc.schema",
            "collaboration.hello",
            "collaboration.me",
            "collaboration.subscribe",
            "collaboration.snapshot",
            "collaboration.resume",
        ] {
            router.set_remote_policy(name, policy::RemotePolicy::Authenticated);
        }
        for name in ["workflows.list", "workflows.get"] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::WorkflowsRead),
            );
        }
        // Snapshot after every module (including rpc.*) registered so
        // discovery lists the discovery methods themselves. The lock is
        // local to this call and no registered handler can run while the
        // router is mutably borrowed, so poisoning is impossible here.
        *registry.write().expect("method registry poisoned") = MethodRegistry::snapshot(router);
    }
}
