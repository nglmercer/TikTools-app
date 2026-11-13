---
title: "Install and manage plugins"
docId: "user.plugins.index"
translationKey: "user.plugins.index"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "93c8811682d0baa5bd18b571503fce483cf1f464e94b94bc933241bf277d23d4"
translationRevision: "93c8811682d0baa5bd18b571503fce483cf1f464e94b94bc933241bf277d23d4"
order: 10
---

# Install and manage plugins {#install-and-manage-plugins}

## Goal and prerequisites {#goal-and-prerequisites}

Add an integration, review its requested access and confirm that its actions are usable. You need a desktop package compatible with the plugin's operating system, architecture and host version. Remote catalog and download operations require network access; some account-based services also require [Cloud sign-in](../cloud/account.md).

## Review before enabling {#review-before-enabling}

1. Open **Plugins** and inspect Installed, Explore or Updates. Explore requests plugin entries only. Find automation templates and profiles in [Automations](../automations/templates.md).
2. Select a plugin to inspect its description, version, compatibility and runtime. Check the publisher and requested capabilities or permissions rather than trusting a familiar display name alone.
3. Use the install flow to inspect the package preview. Review the exact version, signature information, requested access and changes from an installed version before confirming installation.
4. Wait for the package job result. Downloading, installing and enabling are distinct stages; a catalog card alone does not mean the runtime is ready.
5. Open the plugin's Settings or Connection pane. Supply its service address and credentials only in the intended fields. Use a declared connection check when offered.
6. Enable the plugin, then check whether its action types or pages appear. Create a narrowly scoped [logging rule](../automations/first-rule.md) before attaching a powerful external action.

Plugins can run native code or separate processes. A process boundary can isolate a crash, but it is not an operating system sandbox. Install software from publishers you trust. A valid signature identifies reviewed package provenance; it does not make every effect appropriate for your workflow.

## Expected result and status {#expected-result-and-status}

Package review, installation jobs, plugin preferences and version management are available. Actual actions depend on the installed manifest, host capabilities and external service. Event processors have their own panel and can add optional analysis fields to events; raw fields remain distinct.

## Recovery {#recovery}

For installation failure, read the job error for compatibility, signature, runtime or download problems. Do not weaken verification just to clear an error. If an update causes trouble, inspect retained versions and use rollback where available. Review dependent automations before disabling or uninstalling a plugin. Keep [backups](../preferences/backups.md) of plugin data and configuration before removing them.

<DemoFrame scenario="plugins" locale="en" tour="plugin" title="Demo: plugins" />
