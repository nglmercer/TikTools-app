import { $ } from 'bun';
import { cp, mkdir, stat } from 'node:fs/promises';
import { join } from 'node:path';

export const GATEWAY_WIDGET_KINDS = ['follow', 'gift', 'chat', 'share', 'subscribe', 'likes', 'ranking', 'cloud-overlay'] as const;
export type GatewayWidgetKind = (typeof GATEWAY_WIDGET_KINDS)[number];

/** Plugin id of the Event Gateway, which serves the TikTools OBS widgets. */
export const GATEWAY_PLUGIN_ID = 'tiktools.event-gateway';

/**
 * The OBS widgets are TikTools sources (`src/widgets`, the public widget
 * SDK); the Event Gateway plugin lives in the external plugin repository and
 * only serves them. Builds the bundles and copies them into a staged gateway
 * package (`<packageDirectory>/dist/widgets/`), then verifies every entry
 * page, so a gateway can never stage with `/widgets/*` returning 404.
 */
export async function stageGatewayWidgets(repositoryRoot: string, packageDirectory: string): Promise<string> {
  await $`bun scripts/task.ts widgets`.cwd(repositoryRoot).quiet();
  const source = join(repositoryRoot, 'dist', 'widgets');
  await assertWidgetBundle(source);
  const staged = join(packageDirectory, 'dist', 'widgets');
  await mkdir(staged, { recursive: true });
  await cp(source, staged, { recursive: true });
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
        'Run `bun scripts/task.ts widgets` (plugin staging and packaging do this automatically).',
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
