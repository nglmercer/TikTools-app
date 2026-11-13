---
title: "Find preferences and keyboard controls"
docId: "user.preferences.index"
translationKey: "user.preferences.index"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6d9be8451b4db59dfee385e47005777239fa8b2ae249942e6986675a68deb942"
translationRevision: "6d9be8451b4db59dfee385e47005777239fa8b2ae249942e6986675a68deb942"
order: 10
---

# Find preferences and keyboard controls

## Goal and prerequisites

Set the application language, find the correct configuration area and use supported keyboard controls comfortably. You need the desktop window open; network access is only required for the service being configured.

## Find the relevant setting

1. Open **Settings → General** to choose the interface language. Current choices are English (`en`), Spanish (`es`), Portuguese (`pt`) and French (`fr`). Language selection does not change TikTok event identifiers or plugin API names.
2. Review the updater controls in General: channel, automatic checks, notifications and a manual check when supported. Automatic checks are optional. Signed update availability depends on the build's published signing configuration.
3. Use **Account** for Cloud identity and its diagnostics. Use **Collaboration** for owner access configuration, and **Teams** when the host exposes that capability.
4. Open **Plugins** for plugin-wide preferences and each plugin's Settings/Connection panes. A plugin's service credentials do not belong in General settings.
5. Open **Automations → Variables** for shared runtime values, **Points** for earning rules and **Overlays** for widget design. These configuration surfaces are intentionally located beside the features they affect.

Automatic installation is experimental and disabled by the current release workflow's build gates. A visible automatic-check preference does not promise automatic installation. Use the documented [update process](../updates.md) and make a [backup](backups.md) before a version change.

## Use supported keyboard controls

Use Tab and Shift+Tab to move focus through ordinary controls and Enter/Space to activate focused buttons. In the overlay design inspector, Ctrl/Meta+Z undoes and Ctrl/Meta+Shift+Z or Ctrl/Meta+Y redoes. Text inputs retain their own undo behavior. These are editor shortcuts, not global automation hotkeys.

Escape closes the narrow navigation drawer when another control has not consumed it. Dialogs and editors may handle Escape themselves, including an unsaved-change prompt. Global hotkey automation requires a compatible installed plugin and its reported backend capabilities.

## Expected result, status and recovery

These settings and local keyboard interactions are available; complete accessibility/platform acceptance is not claimed. Prefer labeled buttons, readable window sizes and paused feed scrolling when movement is distracting. If a setting appears missing, check its owning page and host capability before resetting data. See [troubleshooting](troubleshooting.md) for unavailable controls and [FAQ](faq.md) for common distinctions.

<DemoFrame scenario="settings" locale="en" title="Demo: settings" />
