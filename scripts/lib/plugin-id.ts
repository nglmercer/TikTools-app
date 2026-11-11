/**
 * Plugin id contract of the host (`crates/tiktools-plugin-api`,
 * `is_valid_plugin_id`) and of `packages/plugin-sdk/plugin.schema.json`:
 * 2 to 128 ASCII characters, lowercase letter first, then lowercase letters,
 * digits, `.`, `_` or `-`.
 *
 * Pure and runtime neutral (no Bun or Node imports) so Vite configuration,
 * staging and packaging all share the same check. An invalid id is never
 * normalized: callers reject it and name the manifest it came from.
 *
 * Ids are not directory selectors (`test plugin <name>`, `--plugin <dir>`)
 * and are not artifact file name segments such as versions; those keep their
 * own, looser, checks.
 */
export const PLUGIN_ID_PATTERN = /^[a-z][a-z0-9._-]{1,127}$/;

export function isValidPluginId(id: unknown): id is string {
  return typeof id === 'string' && PLUGIN_ID_PATTERN.test(id);
}

/** Returns `id` when valid; otherwise throws an error naming `source` (the manifest path). */
export function assertPluginId(id: unknown, source: string): string {
  if (!isValidPluginId(id)) {
    const shown = typeof id === 'string' ? JSON.stringify(id) : String(id);
    throw new Error(
      `${source} has an invalid plugin id ${shown}: expected 2-128 characters matching ${PLUGIN_ID_PATTERN.source}`,
    );
  }
  return id;
}
