# napi-vm-native fixture

Native-addon fixture for `tests/napi_vm_native.rs`: a bundled `rdev-node`
package (standing in for a real device package) consumed by the guest as:

```ts
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { startListener, stopListener } = require('rdev-node');
```

TikTools selects the exact host `.node` from the declared package root
and exposes it as a native `require()` alias under the package name.
Guests never execute package loader code: the fixture package ships no
`index.js`, and the test additionally stages a throwing `index.js` to
prove it stays unexecuted.

## Files

- `node_modules/rdev-node/package.json`: minimal native-only manifest
  (name, version, description). No `main` or `exports`: there is no
  JavaScript entry.
- `node_modules/rdev-node/index.d.ts`: fixture types for `tsc`.
- `src/index.ts` / `dist/index.js`: the guest plugin. Rebuild with:

```bash
node_modules/.bin/tsc -p crates/tiktools-plugin-loader/tests/fixtures/napi-vm-native/tsconfig.json
```

## Staged at test time (never committed)

The integration test copies this directory, builds the real `.node`
from `tests/fixtures/native-tsfn` for the exact host target, writes
placeholder bytes for foreign targets, stages the package through the
SDK staging library, and injects a `{ package, root }` `nativeAddons`
declaration.
