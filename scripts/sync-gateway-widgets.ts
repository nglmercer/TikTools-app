import { cp, mkdir, stat } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const repositoryRoot = resolve(import.meta.dir, '..');
const source = join(repositoryRoot, 'dist', 'widgets');
const target = join(repositoryRoot, 'examples', 'event-gateway-process-plugin', 'dist', 'widgets');

async function exists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

for (const widget of ['follow', 'gift']) {
  if (!(await exists(join(source, widget, 'index.html')))) {
    throw new Error(
      `Widget assets missing at ${source}/${widget}/index.html. Run \`bun run build:widgets\` first.`,
    );
  }
}

await mkdir(target, { recursive: true });
await cp(source, target, { recursive: true });
console.log(`Synced widget assets to ${target}`);
console.log('The event-gateway packager and dev staging pick up dist/ automatically.');
