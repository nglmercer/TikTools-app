---
title: "Releases, packages and updater gates"
docId: "developer.releases"
translationKey: "developer.releases"
locale: "en"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "e28f094fd59897eee4cc337e63a565849b521c329842e96dddaf891e521d0378"
translationRevision: "e28f094fd59897eee4cc337e63a565849b521c329842e96dddaf891e521d0378"
order: 220
---

# Releases, packages and updater gates {#releases-packages-and-updater-gates}

## Goal and prerequisites {#goal-and-prerequisites}

Prepare a reviewable release artifact and distinguish verified downloads from automatic installation. This is a maintainer workflow requiring the appropriate platform toolchains and protected signing material. Ordinary contributors can inspect packaging and version checks without publishing or creating a release tag.

## Prepare consistent artifacts {#prepare-consistent-artifacts}

`package.json`, workspace `Cargo.toml` and `Cargo.lock` share the application version. Use the task runner rather than editing one version independently:

```sh
bun scripts/task.ts version check
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
```

A maintainer's intentional version change uses `bun scripts/task.ts version bump v4.2.3` with the selected next version. The full build also creates widget assets. Release packaging places the native executable beside `web/index.html` and publishes checksums. Installer workflows additionally build NSIS, DEB and AppImage and verify their resources against portable staging. Plugin packages remain separate runtime installations.

## Review updater trust {#review-updater-trust}

`tiktools-app-update` uses signed channel metadata plus signed package bytes. Trusted public key, feed enablement and published verified assets are independent requirements. Package SHA256 sums alone do not enable the app updater. Keep app update keys distinct from plugin publisher keys.

The host limits downloads and redirects, and installation handoff must wait for safe core/WebView shutdown. AppImage installation additionally requires `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` at build time. NSIS needs its own `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1` gate. Existing code and a signature unit test do not justify enabling either gate in release builds.

## Expected result and status {#expected-result-and-status}

Portable packaging and platform installer tooling are implemented. Checks/downloads depend on provisioning; automatic replacement and restart remain experimental. Unknown/extracted Linux layouts fall back to manual installation. A launched NSIS installer does not let the exiting old process report installer success.

## Verify and recover {#verify-and-recover}

Run `bun scripts/task.ts rust test -p tiktools-app-update --locked` for local protocol/signature fixtures. Real upgrade, failure, profile preservation and restart acceptance need disposable supported installations. Keep the previous verified artifact available; an interrupted feed/sidecar publication must fail closed. Do not re-enable handoff when plugin shutdown remains uncertain. The [release workflow](https://github.com/nglmercer/TikTools-app/blob/remake/.github/workflows/release.yml) and updater source own current gates; historical release plans are not a deployment checklist.

## Operational details {#operational-details}

Local portable packaging requires matching source/manifest/binary version, built widgets/web and the release desktop executable. The example uses the current source version; replace the tag when intentionally changing versions. `package` validates the executable and archive layout and writes `release/TikTools-<version>-<platform>.tar.gz` (Linux) or `.zip` (Windows). `RELEASE_BINARY` can select an explicit binary. On Windows use native PowerShell environment assignments and `windows-x86_64`. Installer packaging additionally needs the platform dependencies and pinned cargo-packager from the release workflow, then `bun scripts/package-installers.ts` with the same tag/platform. Portable packaging alone does not produce NSIS/DEB/AppImage.

Compiled updater conjunction: checks require `TIKTOOLS_APP_UPDATE_FEED_ENABLED=1` AND valid `TIKTOOLS_APP_UPDATE_PUBLIC_KEY`; AppImage install also requires `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` AND detected supported source; NSIS additionally requires `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1`. These are build-time values. The release workflow maps repository `TIKTOOLS_UPDATE_PUBLIC_KEY` to the build key/feed gate. Without it, assets/checksums can publish without a signed updater feed. With it, feed publication requires protected signing key/password, signed exact manifest and package bytes, verified published assets and authenticated promotion. Updater signatures are distinct from OS installer signing and plugin publisher keys.

Default updater tests omit real AppImage acceptance. The two ignored tests require a disposable real AppImage fixture reporting version 4.0.0 (the current fixture assertion), FUSE, and cargo-packager signer; replacement also needs a display. Set `TIKTOOLS_TEST_APPIMAGE`, list tests, then deliberately run `preflight_runs_a_real_appimage` and `packager_replaces_a_running_disposable_appimage_without_touching_its_profile` with `-- --ignored --nocapture`. Do not substitute an extracted tree: mount detection differs. Observe replacement, version, restart and preserved profile independently. NSIS needs its own post-exit acceptance; the old process cannot certify installer success.

```sh
bun scripts/task.ts widgets
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
RELEASE_TAG=v4.2.2 RELEASE_PLATFORM=linux-x86_64 bun scripts/task.ts package
```

```powershell
$env:RELEASE_TAG = 'v4.2.2'
$env:RELEASE_PLATFORM = 'windows-x86_64'
bun scripts/task.ts package
```

```sh
TIKTOOLS_TEST_APPIMAGE=/absolute/path/disposable-v4.0.0.AppImage bun scripts/task.ts rust test -p tiktools-app-update --locked preflight_runs_a_real_appimage -- --ignored --nocapture
```
