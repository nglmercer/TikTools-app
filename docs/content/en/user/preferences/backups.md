---
title: "Back up and restore local work"
docId: "user.preferences.backups"
translationKey: "user.preferences.backups"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f4358c358bebe5031f73ba3a14def25c1daa01eb4d7e15f0aabc2a29033ab609"
translationRevision: "f4358c358bebe5031f73ba3a14def25c1daa01eb4d7e15f0aabc2a29033ab609"
order: 20
---

# Back up and restore local work

## Goal and prerequisites

Preserve a working setup before updates, resets or moving computers. You need storage you control and time to close TikTools cleanly. There is no universal **Back up everything** button in the current UI; use the specific exports and a closed-app data copy.

## Export reusable definitions

1. In **Automations**, select the intended profile and use its export action to save a `.tikprofile.json` file. Name it with the app version and date.
2. For a single reusable template, use the template dialog's download/export surface to save `.tiktemplate.json` where offered.
3. Inspect the exported file before sharing it. Templates can contain destinations and configuration values. A profile export is not a copy of balances, analytics, runtime variables, media files or account credentials.

## Copy the local data

1. Stop LIVE activity and any collaboration serving, then exit TikTools normally. Wait until the host has stopped writing.
2. Locate the active data root. Defaults are `%LOCALAPPDATA%\TikTools` on Windows and `~/.local/share/TikTools` on Linux, or `TikTools` beneath `XDG_DATA_HOME` when configured. A custom `TIKTOOLS_HOME` or per-directory override changes the location.
3. Copy the root and any separately overridden data, plugin, plugin-data and plugin-store directories to a dated backup folder. Copy the complete closed database directory, including any SQLite sidecar files present, rather than selecting only one `.db` file.
4. Preserve the referenced audio files separately: audio actions keep paths instead of importing sound bytes. Record the app version and custom directory locations with the backup.
5. Protect the copy because local configuration can contain private values. Cloud authentication lives in the operating system credential store and is not transported by this folder copy.

## Restore and verify

With TikTools closed, preserve the current folder as a separate recovery copy. Restore the saved files to the intended paths and start a compatible version. Inspect profile selection, balances, variables and media paths before reconnecting. For portable definitions, import the exported profile/template through Automations; imported definitions remain disabled until explicit activation.

## Expected result, status and recovery

Closed-app copies and the named import/export surfaces are available recovery methods, without a universal cross-version restore guarantee. Cloud Changes is not a full backup. If a restored version rejects data, stop and return to the preserved copy; reinstalling an older binary does not undo a database migration. See [updates](../updates.md) and [troubleshooting](troubleshooting.md).
