import type { ActionTypeDefinition } from '../../../../automation/behavior/types.ts';

/** Suffix of the self-check action convention: `<anything>.test_connection`. */
export const TEST_CONNECTION_SUFFIX = '.test_connection';

/**
 * Returns the id of the plugin's self-check action (`*.test_connection`),
 * or null when the plugin declares none. The settings pane uses it for the
 * one-click connection check, so validating plain connectivity never needs
 * an automation.
 */
export function findTestConnectionAction(
  actionTypes: readonly ActionTypeDefinition[],
  pluginId: string,
): string | null {
  const match = actionTypes.find(
    (type) =>
      type.source.kind === 'plugin' &&
      type.source.pluginId === pluginId &&
      type.id.endsWith(TEST_CONNECTION_SUFFIX),
  );
  return match?.id ?? null;
}
