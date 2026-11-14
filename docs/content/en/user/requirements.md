---
title: "Requirements"
docId: "user.start.requirements"
translationKey: "user.start.requirements"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "c68c149552a943cf0f03393c5dac91bd62abf9c50f791d7b0757599754f4804e"
translationRevision: "c68c149552a943cf0f03393c5dac91bd62abf9c50f791d7b0757599754f4804e"
order: 10
---

# Requirements {#requirements}

## Goal {#goal}

Choose a supported desktop package and identify the services needed for your intended workflow before installing.

## What you need {#what-you-need}

- A Windows x86_64 or Linux x86_64 computer for the published desktop packages. The current release workflow builds these two targets. It does not publish a macOS desktop or Android APK.
- A working graphical session. Windows needs Microsoft's WebView2 Runtime. Linux needs a compatible GTK/WebKitGTK environment; the DEB declares its runtime packages and the AppImage carries reviewed media helpers.
- Network access to TikTok for an actual LIVE connection. An offline room, expired guest session or upstream rate limit cannot be corrected by changing an automation.
- Storage outside the installation directory for SQLite data, plugins and logs. The app resolves a per-user data directory; moving the executable does not move that data.

A Cloud account is needed for account-based catalog or collaboration operations, not for every local editor. OBS is needed only if you want to display a widget in a broadcast. Audio/TTS integrations may require a separately installed plugin and its own provider or service.

## Check the environment {#check-the-environment}

1. Select the package matching your operating system and x86_64 architecture.
2. On Windows, install WebView2 if the installer reports it missing. On Linux, resolve missing runtime libraries through your distribution package manager.
3. Confirm the app can open its window before connecting to TikTok or adding plugins.
4. Continue with [installation](installation.md) and then [your first LIVE](first-live.md).

## Expected result {#expected-result}

The application opens and its main navigation is usable while offline. Optional integrations may remain unavailable until their plugin or host capability is present.

## Recovery {#recovery}

For a blank window or an early exit, collect startup diagnostics and the exact package version. The AppImage and DEB are different installation formats; neither is an Android executable. See [Termux and headless scope](termux.md) for that environment. Building from source has additional requirements in the developer guide.

## Operational details {#operational-details}

If the window never opens, use native `tiktools.log` (`controller.log` for controller mode): Windows `%LOCALAPPDATA%/TikTools/logs`, Linux `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs`, unless `TIKTOOLS_HOME` or `TIKTOOLS_LOG_DIR` changes the location. Logging initialization can fail before a file exists. Retain any terminal error and follow [diagnostic retention and privacy](preferences/troubleshooting.md#operational-details) before sharing.
