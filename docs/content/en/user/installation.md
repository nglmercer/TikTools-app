---
title: "Install Windows or Linux"
docId: "user.start.installation"
translationKey: "user.start.installation"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6f8673490bebb3447474a70d914d06102de87a9ef2f3f3c51da897d0d2cc8c1c"
translationRevision: "6f8673490bebb3447474a70d914d06102de87a9ef2f3f3c51da897d0d2cc8c1c"
order: 20
---

# Install Windows or Linux {#install-windows-or-linux}

## Goal and prerequisites {#goal-and-prerequisites}

Install a published desktop package without setting up a compiler. Check the [requirements](requirements.md), close any running TikTools instance, and download from the project's [GitHub releases](https://github.com/nglmercer/TikTools-app/releases).

## Choose a package {#choose-a-package}

| Package | Use |
| --- | --- |
| Windows `installer.exe` | Per-user NSIS installation; WebView2 is required. |
| Linux `installer.AppImage` | A portable executable image for a compatible Linux desktop. |
| Linux `installer.deb` | Installation through a Debian/Ubuntu package manager. |
| Windows ZIP / Linux TAR.GZ | Portable directory containing the executable and its resources. |

Read whether a release is stable or a prerelease. Prereleases are debugging candidates, not automatic promises of production compatibility. Use the release's `SHA256SUMS.txt` to check downloaded bytes before replacing an existing installation.

## Windows {#windows}

1. Download the x86_64 installer and verify its checksum.
2. Run the installer under the user account that will run TikTools. Install WebView2 if requested.
3. Launch TikTools. Open **Settings → General** to select the interface language.
4. For a portable ZIP, extract the whole directory. Keep `web/` and other packaged resources beside the executable; do not run only a copied EXE.

## Linux {#linux}

1. Download the matching AppImage, DEB or portable archive and verify its checksum.
2. For AppImage, allow execution in the file manager or run `chmod +x` on the downloaded file, then open it.
3. For DEB, install through your system package manager so its runtime dependencies are resolved.
4. For TAR.GZ, extract the full directory and launch its desktop executable with the packaged resources intact.

## Expected result {#expected-result}

TikTools opens in an offline state. Data is stored separately from the installation, normally under `%LOCALAPPDATA%/TikTools` on Windows or `~/.local/share/TikTools` on Linux.

## Recovery {#recovery}

If the package fails before a window appears, check diagnostics and missing library errors before modifying account or migration settings. Back up the data directory with the app stopped before trying another version. Reinstalling an older executable does not reverse database migrations. Continue with [your first LIVE](first-live.md) or [updates](updates.md).

## Operational details {#operational-details}

If the window never opens, use native `tiktools.log` (`controller.log` for controller mode): Windows `%LOCALAPPDATA%/TikTools/logs`, Linux `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs`, unless `TIKTOOLS_HOME` or `TIKTOOLS_LOG_DIR` changes the location. Logging initialization can fail before a file exists. Retain any terminal error and follow [diagnostic retention and privacy](preferences/troubleshooting.md#operational-details) before sharing.
