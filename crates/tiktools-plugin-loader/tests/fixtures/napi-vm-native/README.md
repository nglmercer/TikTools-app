# napi-vm-native fixture

Native-addon fixture for `tests/napi_vm_native.rs`: a bundled `rdev-node`
package (standing in for a real device package) consumed by the guest as:

```ts
import { startListener, stopListener } from 'rdev-node';
```

## Generated files

`node_modules/rdev-node/index.js` is the byte output of the real
`nglmercer/napi-rs` direct/bundled loader generator
(`createDirectMultiCjsBinding` over the eight desktop triples, idents
`add`, `startListener`, `stopListener`, fork rev
`25d21dbfb679773589355f3c0529736faeb23317`). It is not hand-written:
the equivalent CLI invocation is:

```bash
napi build --platform \
  --target x86_64-unknown-linux-gnu \
  --binding-loader direct \
  --release
```

with all eight desktop triples in `napi.targets`. Every CI job emits the
identical file; only the host `.node` differs per job. Do not edit the
generated loader by hand — regenerate it.

## Hand-written files

- `wrapper.mjs`: tiny ESM bridge. napi-vm resolves static ESM imports to
  JavaScript sources only, so the generated CommonJS loader cannot be
  imported directly; the wrapper re-exports it through the VM's
  `require()`.
- `index.d.ts`: fixture types for `tsc` (the guest compiles with the
  repository TypeScript compiler; napi-vm never transpiles).
- `src/index.ts` / `dist/index.js`: the guest plugin. Rebuild with:

```bash
node_modules/.bin/tsc -p crates/tiktools-plugin-loader/tests/fixtures/napi-vm-native/tsconfig.json
```

## Staged at test time (never committed)

The integration test copies this directory, builds the real `.node`
from `tests/fixtures/native-tsfn` for the exact host target, writes
placeholder bytes for foreign targets, and injects `nativeAddons` with
computed SHA-256 digests.
