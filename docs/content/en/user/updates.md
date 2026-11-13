---
title: "Downloads and updates"
docId: "user.start.updates"
translationKey: "user.start.updates"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "53a9e9209749f0d6ea58737a3a2eadf871ad344e41fa17cd0e363cf43e181b3f"
translationRevision: "53a9e9209749f0d6ea58737a3a2eadf871ad344e41fa17cd0e363cf43e181b3f"
order: 40
---

# Downloads and updates

## Goal and prerequisites

Choose the right release channel and update without confusing an update check with an automatic installation. Close active sessions and back up local data before changing versions, especially across a documented database migration.

## Select a channel

The [release list](https://github.com/nglmercer/TikTools-app/releases) identifies stable versions and prereleases. A candidate such as `-rc` is intended for verification. The application updater distinguishes stable and beta feeds; it authenticates signed metadata before offering a version. An unsigned JSON file or a changed installer is not an accepted update.

## Check and install

1. Open **Settings → General** and inspect the updater's current state and installed version.
2. Run the available check. Builds without the configured public key/feed capability show that checks are unavailable.
3. Read the release notes and choose the package for your platform.
4. When the build offers a download/manual handoff, follow that path. The production release workflow enables authenticated checks; it does not enable every experimental automatic-install feature.
5. For a manual update, close TikTools, replace or install the complete verified package, and reopen it. Keep the data directory separate from the application files.

## Expected result

The restarted app reports the chosen version and can read the expected local profile. Portable ZIP/TAR packages require manual replacement. Unknown or extracted layouts may also require a manual path even when an AppImage installer exists.

## Recovery and limitations

A failed signature check is a failed update; download a fresh official copy and verify the release checksums. Do not bypass verification. Automatic AppImage replacement and NSIS handoff have separate build gates and are experimental. There is no general automatic rollback. Returning to an older binary does not undo a migrated SQLite schema.

Report the installed version, package format and updater error from Diagnostics. Do not attach signing keys, invitation tokens or account data. The [installation guide](installation.md) describes the supported package layouts.
