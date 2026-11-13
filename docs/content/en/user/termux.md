---
title: "Termux and headless scope"
docId: "user.start.termux"
translationKey: "user.start.termux"
locale: "en"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a823f6d8f669895af6858723159b7eb4fea1fbaf165102b61396b4d2e90efa19"
translationRevision: "a823f6d8f669895af6858723159b7eb4fea1fbaf165102b61396b4d2e90efa19"
order: 50
---

# Termux and headless scope

## Goal and prerequisites

Understand which parts of TikTools work without a desktop WebView. This is an advanced workflow, not an Android version of the Windows/Linux installer. The project has a native Rust CLI for Termux and a headless host; obtaining a matching binary or building it belongs to the developer setup.

## Use one host

1. Start one headless host with `tiktools host --ipc` in the environment where the CLI was installed.
2. Keep its application home consistent between terminals.
3. From another terminal, use `tiktools system info`, `tiktools live status` or `tiktools api discover` to inspect that host.
4. Stop it cleanly with the documented shutdown command or Ctrl-C. Do not remove ownership locks while it is running.

Ordinary CLI commands connect to the existing host. `--standalone` explicitly creates a separate core for one invocation; it is not the default way to control an already-running host.

## Local browser preview

The optional `local-web` adapter can serve the Vue interface to a browser after local pairing. It is a development preview with separate build and lifecycle requirements. It does not make Android a supported desktop WebView target, and it does not grant a remote person access to the host.

## Expected result

The CLI and desktop share control concepts, SQLite persistence and native event processing. Platform-dependent plugins, audio, hotkeys, native windows and credential stores may be unavailable. Native remote profile persistence currently has explicit Android restrictions.

## Recovery

Use Termux's private app directory rather than Android shared storage, which can prevent execution and Unix sockets. Keep the same application home and ensure the host is alive before debugging an IPC failure. If Android stops the process, restart it with that same profile and inspect recovery diagnostics. Do not expect the documentation demo to reproduce Android lifecycle behavior.

The [developer guide](../developer/index.md) contains the actual toolchain, feature flags and on-device verification procedures. Desktop binary installation remains in the [Windows/Linux guide](installation.md).
