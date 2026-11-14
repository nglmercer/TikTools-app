---
title: "Install Windows or Linux"
docId: "user.start.installation"
translationKey: "user.start.installation"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "ed9e7c1bcb88d991ed3d8ea8a399aa5b5e9ca8cfa58de580f8ca757344d26ca2"
translationRevision: "ed9e7c1bcb88d991ed3d8ea8a399aa5b5e9ca8cfa58de580f8ca757344d26ca2"
order: 20
---

# Install Windows or Linux {#install-windows-or-linux}

## Goal and prerequisites {#goal-and-prerequisites}

Install a published desktop package without setting up a compiler. Download from the project's [GitHub releases](https://github.com/nglmercer/TikTools-app/releases), then follow the instructions for your operating system.

<DocRequirements title="Before you start">

- Check the [system requirements](requirements.md) and choose a compatible package for your computer.
- Close any running TikTools instance before replacing the application.
- Read the release notes and keep the release's `SHA256SUMS.txt` available to verify your download.
- Before trying another version, back up the data directory with the app stopped.

</DocRequirements>

## Choose a package {#choose-a-package}

<DocTabs label="Choose your system">

<template #windows>

Choose the x86_64 **`installer.exe`** for a per-user NSIS installation. The desktop application requires **WebView2**; install it if the installer requests it.

Use the **ZIP** package for a portable directory. Extract the whole archive and keep the executable, `web/` and all other packaged resources together.

[Follow the Windows instructions](#windows)

</template>

<template #linux>

Choose **`installer.AppImage`** for a portable executable image on a compatible Linux desktop, **`installer.deb`** for installation through a Debian/Ubuntu package manager, or **TAR.GZ** for a portable directory containing the executable and its resources.

AppImage needs execution permission. DEB uses your package manager to resolve runtime dependencies. Extract the entire TAR.GZ directory and keep all packaged resources together.

[Follow the Linux instructions](#linux)

</template>

</DocTabs>

Read whether a release is stable or a prerelease. Prereleases are debugging candidates, not automatic promises of production compatibility. Use the release's `SHA256SUMS.txt` to check downloaded bytes before replacing an existing installation.

## Windows {#windows}

<DocSteps>

1. Download the x86_64 installer and verify its checksum.
2. Run the installer under the user account that will run TikTools. Install WebView2 if requested.
3. Launch TikTools. Open **Settings → General** to select the interface language.
4. For a portable ZIP, extract the whole directory. Keep `web/` and other packaged resources beside the executable; do not run only a copied EXE.

</DocSteps>

## Linux {#linux}

<DocSteps>

1. Download the matching AppImage, DEB or portable archive and verify its checksum.
2. For AppImage, allow execution in the file manager or run `chmod +x` on the downloaded file, then open it.
3. For DEB, install through your system package manager so its runtime dependencies are resolved.
4. For TAR.GZ, extract the full directory and launch its desktop executable with the packaged resources intact.

</DocSteps>

## Expected result {#expected-result}

TikTools opens in an offline state. Data is stored separately from the installation, normally under `%LOCALAPPDATA%/TikTools` on Windows or `~/.local/share/TikTools` on Linux.

## Recovery {#recovery}

If the package fails before a window appears, check diagnostics and missing library errors before modifying account or migration settings. Back up the data directory with the app stopped before trying another version. Reinstalling an older executable does not reverse database migrations. Continue with [your first LIVE](first-live.md) or [updates](updates.md).

## Operational details {#operational-details}

If the window never opens, use native `tiktools.log` (`controller.log` for controller mode): Windows `%LOCALAPPDATA%/TikTools/logs`, Linux `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs`, unless `TIKTOOLS_HOME` or `TIKTOOLS_LOG_DIR` changes the location. Logging initialization can fail before a file exists. Retain any terminal error and follow [diagnostic retention and privacy](preferences/troubleshooting.md#operational-details) before sharing.
