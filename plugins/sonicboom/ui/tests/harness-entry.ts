/**
 * Interop harness entry (test-only): exposes the REAL host shim on the
 * harness page. Bundled with `bun build --format=iife` by
 * `interop.spec.ts`; never shipped with the plugin package.
 */
import * as shim from '../../../../src/web/plugin-ui/plugin-webview-host.ts';

(window as unknown as { TikToolsShim: typeof shim }).TikToolsShim = shim;
