# Desktop packaging (in development)

The release workflow stages portable ZIP and TAR.GZ archives and builds NSIS,
DEB, and AppImage installers. It checks extracted installer resources against
the portable staging tree and runs the extracted executable's version command.
Installer upgrades and normal startup still require real Windows and Linux
smoke tests; package extraction alone does not establish that they work.

`Packager.toml` uses cargo-packager 0.11.8 and reuses
`release/staging/TikTools`, which the existing portable packaging task creates.
Run the normal frontend and release binary builds, then run
`RELEASE_TAG=v4.0.0 RELEASE_PLATFORM=linux-x86_64 bun scripts/task.ts package`
(use `windows-x86_64` on Windows). This stages and verifies the frontend and
built-in plugins once and creates the portable archive. Set the tag to the
actual release version; all version files and the compiled executable must
match it. CI checks these before staging.

Before running Packager, explicitly check the **staged** binary as well:

```sh
RELEASE_TAG=v4.0.0 RELEASE_BINARY=release/staging/TikTools/tiktools-desktop \
  bun scripts/check-version.ts --binary
NO_STRIP=1 cargo packager --config Packager.toml --formats deb,appimage
```

Alternatively, after the portable staging task succeeds, run
`RELEASE_TAG=v4.0.0 RELEASE_PLATFORM=linux-x86_64 bun scripts/package-installers.ts`.
This checks the staged executable's reported version, requires Packager 0.11.8,
compares extracted contents with staged assets, and writes versioned installers
without overwriting existing outputs. AppImage ELF files can be changed by
linuxdeploy; for those files it checks their GNU build IDs and executable version
instead of byte equality. This is a packaging check, not a cryptographic
signature check. It sets
`NO_STRIP=1` because linuxdeploy's bundled strip does not support RELR sections
in some current Linux libraries. Windows uses the same command with
`RELEASE_PLATFORM=windows-x86_64` in PowerShell environment variables.

On Windows PowerShell, set `RELEASE_TAG` and `RELEASE_BINARY` in the process
environment, using `release/staging/TikTools/tiktools-desktop.exe` for the
latter, then run the same version check followed by
`cargo packager --config Packager.toml --formats nsis`. The NSIS configuration
selects per-user installation and checks for Microsoft's WebView2 Runtime;
if it is absent, the installer stops and directs the user to Microsoft's
download page. This check and installer behavior have not been tested on
Windows yet.

DEB resources are installed under `/usr/lib/tiktools-desktop`; portable and
Windows resources are adjacent to the executable. AppImage uses the same
relative `usr/lib` layout as the DEB staging step. User data remains in
`AppPaths`, outside the installation directory. The DEB dependency list uses
Debian/Ubuntu package names and alternatives; other Linux distributions use
their own package names and should use a tested AppImage or portable build.
AppImage runtime dependencies and clean-machine startup still need validation.

To migrate from a portable version, close TikTools and install a verified
release through the platform installer once installers are available. The
default data directory is independent of the portable executable, but back up
the data directory first, especially before a database migration. Portable
archives require manual replacement; they are not eligible for automatic
installation. Reinstalling a previous version is a manual recovery option and
does not reverse database schema changes.

The release workflow creates a new draft release, downloads its uploaded assets,
checks their SHA-256 hashes and file list, and only then publishes the draft.
It does not replace assets on an existing release. If publication fails, inspect
and remove the incomplete draft before retrying; never delete a published
historical release to retry. This verification does not activate the updater
feed: signed metadata publication remains a separate, unfinished release gate.
