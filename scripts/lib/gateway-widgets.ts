import { $ } from 'bun';
import { stat } from 'node:fs/promises';
import { join } from 'node:path';

export const GATEWAY_WIDGET_KINDS = ['follow', 'gift', 'chat', 'share', 'subscribe'] as const;
export type GatewayWidgetKind = (typeof GATEWAY_WIDGET_KINDS)[number];

/**
 * Builds all OBS widget bundles and stages them into the Event Gateway
 * package (`dist/widgets/`), then verifies every entry page exists. Used by
 * plugin packaging and dev staging so a gateway can never ship (or stage)
 * with `/widgets/*` returning 404. Throws loudly when anything is missing.
 */
export async function ensureGatewayWidgetsStaged(repositoryRoot: string): Promise<string> {
  await $`bun run build:widgets`.cwd(repositoryRoot).quiet();
  await $`bun run sync:gateway-widgets`.cwd(repositoryRoot).quiet();
  const staged = join(repositoryRoot, 'examples', 'event-gateway-process-plugin', 'dist', 'widgets');
  await assertWidgetBundle(staged);
  return staged;
}

/** Fails when any staged widget entry page is absent. */
export async function assertWidgetBundle(widgetsDir: string): Promise<void> {
  const missing: string[] = [];
  for (const kind of GATEWAY_WIDGET_KINDS) {
    if (!(await exists(join(widgetsDir, kind, 'index.html')))) {
      missing.push(`${kind}/index.html`);
    }
  }
  if (missing.length > 0) {
    throw new Error(
      `Event Gateway widget assets missing under ${widgetsDir}: ${missing.join(', ')}. ` +
        `Run \`bun run build:widgets && bun run sync:gateway-widgets\` (packaging and dev staging do this automatically).`,
    );
  }
}

async function exists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}
