# napi-vm migration plan

Prepared 2026-10-26 after the dependency refresh. This is a separate migration;
the dependency update intentionally retains the existing VM revision.

## Baseline and target

- Current pin: `881cc8f1cec262049b6d31fa499b0d601ba673c5` in root
  `Cargo.toml` and `Cargo.lock`, with `default-features = false`.
- Reviewed upstream main: `3816dcfec05c0d4d5d2e05d20e8cff71e98ac1ee`,
  321 commits ahead of the pin. Pin this exact revision for the migration;
  recheck main and review any additional changes before choosing another target.
- At PR preparation, upstream main advanced to
  `fc2514bb0d3e2ddbc0c9ec66e240c4a170dc2f71`. The reviewed migration target
  above remains the fixed snapshot; audit the newer commits before selecting
  that newer head.
- Latest published GitHub release is still v0.1.5. A release-version comparison
  alone misses the changes consumed through the Git dependency.
- Keep `napi-vm-node-api` forwarding to upstream `node-api-host` and preserve
  the supported-target gates (Linux, macOS, Windows; Android remains fail-closed
  for native addons).

Upstream sources reviewed:

- [Architecture and UTF-16 migration](https://github.com/nglmercer/napi-vm/blob/3816dcfec05c0d4d5d2e05d20e8cff71e98ac1ee/docs/crate-architecture.md)
- [Extraction validation and limitations](https://github.com/nglmercer/napi-vm/blob/3816dcfec05c0d4d5d2e05d20e8cff71e98ac1ee/docs/utf16-extraction-validation.md)
- [Plugin host implementation](https://github.com/nglmercer/napi-vm/blob/3816dcfec05c0d4d5d2e05d20e8cff71e98ac1ee/src/plugin_host.rs)
- [Revision comparison](https://github.com/nglmercer/napi-vm/compare/881cc8f1cec262049b6d31fa499b0d601ba673c5...3816dcfec05c0d4d5d2e05d20e8cff71e98ac1ee)

## Implementation sequence

1. Establish the refreshed dependency baseline. Record successful script-service,
   plugin loader, native-addon and workspace checks on the current pin. Preserve
   the dependency changes independently so regressions can be isolated.
2. Change only the root VM revision and regenerate `Cargo.lock`. Keep the facade
   dependency initially: it reexports the core APIs and still owns `RustPluginHost`.
   A direct dependency on `napi-vm-core` is optional future cleanup, not required
   for this migration.
3. Migrate `crates/tiktools-core/src/services/script.rs`:
   - `VmValue::String` now takes `JsString`; convert Rust strings with `.into()`.
   - Check `Lexer`, parser spans, `get_prop`, object constructors, `set_source`,
     execution and JSON conversion signatures against the selected revision.
   - Preserve the 128 KiB source limit, 2 MiB result limit and loop budget.
   - Choose strict UTF-8 conversion (`to_utf8()`) where a Rust/JSON text contract
     requires it; do not silently replace unpaired surrogates. Keep guest UTF-16
     code units lossless until that boundary.
   - Verify `log()` explicitly. Core initialization now exposes no ambient
     console/timers/Buffer/filesystem/network. Install only an intentional logging
     callback if the script contract requires console; never enable the full
     runtime merely to restore one global.
4. Adapt `crates/tiktools-plugin-loader/src/napi_vm.rs` and its unit tests:
   - Confirm capability callbacks, `value_to_json`/`value_from_json`, error values,
     interpreter access, wake notifications and execution-budget methods compile.
   - Preserve one owner thread per VM, JSON/byte messages across threads, bounded
     queues, startup deadlines and event-driven idle wakeups.
   - Preserve the existing manifest bridge (`apiVersion: 1`); TikTools permissions
     remain authoritative. Do not pass its string-array permissions as upstream's
     differently shaped permissions object.
   - Keep capability defaults denied. Upstream `node-api-host` now forwards to
     `runtime-node`; compiling it must not grant filesystem/network/npm access.
     Review installed plugin globals and `node:module` behavior explicitly.
5. Verify trusted-native integration and fixture compatibility:
   - Retain exact binary selection, SHA-256 authorization, alias authorization,
     trust checks, and undeclared-addon rejection.
   - Verify `RustPluginNapiOptions`, `allow_addon`, `allow_addon_alias`,
     `configure_napi_addons` and `createRequire` against the selected revision.
   - Exercise TSFN delivery while idle, finalizers, active-listener reload,
     shutdown and repeated load/unload. Include snapshot shared-object identity
     regressions from upstream when building a focused compatibility case.
   - The refreshed native fixture uses current napi-rs. Distinguish fixture SDK
     changes from VM regressions by reproducing on both VM revisions.
6. Build and run the shipped plugin examples, including the hotkey addon on a
   supported desktop. Preserve deny-by-default behavior and the plugin JSON
   request/result protocol. Review any newly required guest imports/globals.

## Acceptance gates

Run with the repository's configured toolchains, on Linux and Windows desktop CI.
Android checks are additional evidence, not proof of desktop native-addon support.

```sh
bun run contracts:check
bun run deps:check
bun run check:web
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p tiktools-core services::script --locked
cargo test -p tiktools-plugin-loader --test napi_vm --locked
cargo test -p tiktools-plugin-loader --test napi_vm_native --locked
cargo test -p tiktools-plugin-loader --no-default-features --test napi_vm --locked
cargo test --workspace --locked
cargo test --manifest-path examples/event-gateway-process-plugin/Cargo.toml --locked
bun run scripts/build-plugin.ts --all --debug
cargo build -p tiktools-desktop --release --locked
```

Add focused regressions for emoji, combining characters, lone surrogates,
JSON roundtrips, blocked ambient capabilities, script logging, loop exhaustion,
startup hangs, idle callbacks and reload cleanup where existing tests do not
cover the changed behavior. Do not skip existing failures to make the upgrade
appear green. Compare any failure with the baseline before assigning its cause.

## Rollout and rollback

Deliver the revision/adapter changes in a dedicated PR, referencing the dependency
refresh. Attach baseline and candidate check results, supported desktop native
results and any unresolved upstream limitations. Test a packaged application,
a script automation and a plugin callback through reload before merging.

Rollback means reverting the migration commit (VM pin, lockfile, adapters and
migration-only fixtures together). Keep the dependency refresh intact. The
migration does not require a persistent data or plugin protocol version change;
if implementation discovers one, document and test compatibility before rollout.
