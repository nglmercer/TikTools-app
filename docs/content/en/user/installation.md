---
title: "Install Windows or Linux"
docId: "user.start.installation"
translationKey: "user.start.installation"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "e8ced59a75bcb6d58360ec117f561079657edbe661a37fce6dfb197d2e9a48a0"
translationRevision: "e8ced59a75bcb6d58360ec117f561079657edbe661a37fce6dfb197d2e9a48a0"
order: 20
---

# Install Windows or Linux

## Goal and prerequisites

Install a published desktop package without setting up a compiler. Check the [requirements](requirements.md), close any running TikTools instance, and download from the project's [GitHub releases](https://github.com/nglmercer/TikTools-app/releases).

## Choose a package

| Package | Use |
| --- | --- |
| Windows `installer.exe` | Per-user NSIS installation; WebView2 is required. |
| Linux `installer.AppImage` | A portable executable image for a compatible Linux desktop. |
| Linux `installer.deb` | Installation through a Debian/Ubuntu package manager. |
| Windows ZIP / Linux TAR.GZ | Portable directory containing the executable and its resources. |

Read whether a release is stable or a prerelease. Prereleases are debugging candidates, not automatic promises of production compatibility. Use the release's `SHA256SUMS.txt` to check downloaded bytes before replacing an existing installation.

## Windows

1. Download the x86_64 installer and verify its checksum.
2. Run the installer under the user account that will run TikTools. Install WebView2 if requested.
3. Launch TikTools. Open **Settings → General** to select the interface language.
4. For a portable ZIP, extract the whole directory. Keep `web/` and other packaged resources beside the executable; do not run only a copied EXE.

## Linux

1. Download the matching AppImage, DEB or portable archive and verify its checksum.
2. For AppImage, allow execution in the file manager or run `chmod +x` on the downloaded file, then open it.
3. For DEB, install through your system package manager so its runtime dependencies are resolved.
4. For TAR.GZ, extract the full directory and launch its desktop executable with the packaged resources intact.

## Expected result

TikTools opens in an offline state. Data is stored separately from the installation, normally under `%LOCALAPPDATA%/TikTools` on Windows or `~/.local/share/TikTools` on Linux.

## Recovery

If the package fails before a window appears, check diagnostics and missing library errors before modifying account or migration settings. Back up the data directory with the app stopped before trying another version. Reinstalling an older executable does not reverse database migrations. Continue with [your first LIVE](first-live.md) or [updates](updates.md).
