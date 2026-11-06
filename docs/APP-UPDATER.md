# Application updater (in development)

Application updates are separate from plugin updates. The desktop exposes
status, checks, and verified downloads through the control API and Settings page.
Checks are disabled until a trusted public key is embedded and update manifests
are published. The experimental AppImage handoff is separately disabled in
normal builds, so existing releases must still be updated manually.

The `tiktools-app-update` crate defines stable and beta version policy and reads
the static JSON format used by `cargo-packager-updater` 0.2.3. Stable accepts
only newer stable versions; beta accepts newer stable or prerelease versions.
The proposed static endpoints are `updates/stable.json` and `updates/beta.json`
under the project's GitHub Pages origin. Each JSON document also needs a
`<name>.json.sig` sidecar containing a Packager-format Minisign signature over
the exact JSON bytes. This authenticates the version and platform mapping as
well as the separate signature of each package. A publication interrupted
between the JSON and sidecar uploads will fail signature validation rather
than install an update.

The Rust downloader limits update payloads to 512 MiB, restricts redirects to
HTTPS GitHub release asset hosts, and verifies the package signature using the
same key used for the metadata. The experimental AppImage path requires a
native confirmation, shuts down the core and WebViews, releases the instance
guard and runtime, and then calls Packager to replace the image. It checks the
new image's reported version before launching it. This path requires
`TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` at build time in addition to the feed
gate; do not set it in a release until real install and restart tests pass.
NSIS automatic installation remains unsupported: Packager 0.2.3 exits the
process during Windows installation, and that handoff has not been tested.
On Linux, `tiktools-desktop --update-installation-type` reports the detected
installation type without starting the GUI. AppImage auto-installation requires
the executable to be inside the mounted image and validates the outer image's
header; extracted or unknown layouts fall back to manual installation.

For development, run `cargo test -p tiktools-app-update --locked`. Tests use
public signature vectors and loopback HTTP fixtures; they do not prove that
the production endpoints or operating-system installers work. Create a
dedicated update signing key with `cargo packager signer generate`, store its
private key and password in protected CI secrets, and embed only its public key
in trusted release builds using the compile-time `TIKTOOLS_APP_UPDATE_PUBLIC_KEY`
environment variable. `TIKTOOLS_APP_UPDATE_FEED_ENABLED=1` is a separate build
gate and must only be set after the signed feed has been published and verified.
Do not use a plugin publisher key. Do not publish a
manifest until the corresponding signed assets are uploaded and verified.

After both platform builds have produced verified installer artifacts, the
release tooling can generate and check metadata without publishing anything:

```sh
RELEASE_TAG=v4.0.0 bun scripts/generate-update-manifest.ts --sign release-assets
RELEASE_TAG=v4.0.0 bun scripts/generate-update-manifest.ts --verify release-assets
```

Both commands require `TIKTOOLS_UPDATE_PUBLIC_KEY`. Signing also requires
`CARGO_PACKAGER_SIGN_PRIVATE_KEY` and
`CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD`; supply them through protected
environment secrets, never command-line arguments or logs. The command signs
the NSIS and AppImage files and `update.json`, then verifies all three using the
client's verifier. It rejects missing or mismatched versions and artifacts.
These commands only prepare local files: a protected signing job, upload
verification, and atomic channel publication are still required. Current feed
heads must be authenticated before promotion; stable may advance on stable
versions only, while beta advances only to a higher SemVer version.

The updater key provisioning, manifest publication, NSIS installation, and
end-to-end recovery flows remain to be completed and tested. No
automatic rollback is currently provided. Keep backups of user data before
testing schema-changing releases; reinstalling an older binary does not undo
database migrations.
