/**
 * Decides whether the standalone OBS widget installs the synthetic-event
 * test hook (`window.__tiktoolsWidgetTest`).
 *
 * Production bundles must not expose the hook: it lets any script in the
 * page inject fake follows, gifts, chats, shares, and subscribes. The hook
 * is enabled only by build-time signals — Vite dev mode or the explicit
 * `VITE_TIKTOOLS_WIDGET_TEST_HOOK=1` E2E build flag — never by URL query,
 * hash, storage, or any other user-controlled runtime input.
 */

/** Build-time flag enabling the hook for widget E2E bundles. */
export const WIDGET_TEST_HOOK_ENV = 'VITE_TIKTOOLS_WIDGET_TEST_HOOK';

export interface WidgetTestHookEnv {
  dev?: unknown;
  testHookFlag?: unknown;
}

/**
 * Pure resolver so the default-off behavior stays unit-testable without a
 * Vite build. Only the exact string `'1'` (or boolean `true`) enables the
 * flag; every other value — including URL-shaped strings — stays disabled.
 */
export function resolveWidgetTestHookEnabled(env: WidgetTestHookEnv = {}): boolean {
  if (env.dev === true) return true;
  return env.testHookFlag === '1' || env.testHookFlag === true;
}

/**
 * Reads the current Vite build env. Unknown hosts (Bun tests, plain
 * browsers without Vite defines) fall back to disabled.
 */
export function isWidgetTestHookEnabled(): boolean {
  try {
    const env = (import.meta as unknown as { env?: Record<string, unknown> }).env;
    if (!env || typeof env !== 'object') return false;
    return resolveWidgetTestHookEnabled({
      dev: env['DEV'],
      testHookFlag: env[WIDGET_TEST_HOOK_ENV],
    });
  } catch {
    return false;
  }
}
