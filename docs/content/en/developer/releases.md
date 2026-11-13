---
title: "Releases, packages and updater gates"
docId: "developer.releases"
translationKey: "developer.releases"
locale: "en"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7b1c1c12ef3768dd765301e04b7894670570ae78e72a446ace0b405da157c39a"
translationRevision: "7b1c1c12ef3768dd765301e04b7894670570ae78e72a446ace0b405da157c39a"
order: 220
---

# Releases, packages and updater gates

## Goal and prerequisites

Prepare a reviewable release artifact and distinguish verified downloads from automatic installation. This is a maintainer workflow requiring the appropriate platform toolchains and protected signing material. Ordinary contributors can inspect packaging and version checks without publishing or creating a release tag.

## Prepare consistent artifacts

`package.json`, workspace `Cargo.toml` and `Cargo.lock` share the application version. Use the task runner rather than editing one version independently:

```sh
bun scripts/task.ts version check
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
```

A maintainer's intentional version change uses `bun scripts/task.ts version bump v4.2.3` with the selected next version. The full build also creates widget assets. Release packaging places the native executable beside `web/index.html` and publishes checksums. Installer workflows additionally build NSIS, DEB and AppImage and verify their resources against portable staging. Plugin packages remain separate runtime installations.

## Review updater trust

`tiktools-app-update` uses signed channel metadata plus signed package bytes. Trusted public key, feed enablement and published verified assets are independent requirements. Package SHA256 sums alone do not enable the app updater. Keep app update keys distinct from plugin publisher keys.

The host limits downloads and redirects, and installation handoff must wait for safe core/WebView shutdown. AppImage installation additionally requires `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` at build time. NSIS needs its own `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1` gate. Existing code and a signature unit test do not justify enabling either gate in release builds.

## Expected result and status

Portable packaging and platform installer tooling are implemented. Checks/downloads depend on provisioning; automatic replacement and restart remain experimental. Unknown/extracted Linux layouts fall back to manual installation. A launched NSIS installer does not let the exiting old process report installer success.

## Verify and recover

Run `bun scripts/task.ts rust test -p tiktools-app-update --locked` for local protocol/signature fixtures. Real upgrade, failure, profile preservation and restart acceptance need disposable supported installations. Keep the previous verified artifact available; an interrupted feed/sidecar publication must fail closed. Do not re-enable handoff when plugin shutdown remains uncertain. The [release workflow](https://github.com/nglmercer/TikTools-app/blob/remake/.github/workflows/release.yml) and updater source own current gates; historical release plans are not a deployment checklist.
