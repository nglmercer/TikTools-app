import { cp, mkdir, rm, stat } from 'node:fs/promises';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';

/**
 * Replace the `web/` directory a packaged desktop host reads with a fresh
 * copy of `dist/web`. The release host never falls back to `dist/web`: it
 * loads `<executable directory>/web/index.html` (see
 * `FrontendSource::from_environment` in tiktools-desktop). Without this step
 * a rebuilt binary can keep serving an older bundle, including CSS whose
 * Vue scope ids no longer match the JS that ships with it.
 *
 * Usage: bun scripts/sync-web-assets.ts [target-web-directory] [source-web-directory]
 * Default target: target/release/web (beside the release binary).
 * Default source: dist/web.
 */
const repositoryRoot = resolve(import.meta.dir, '..');
const sourceArgument = process.argv[3]?.trim();
const source = sourceArgument
  ? (isAbsolute(sourceArgument) ? resolve(sourceArgument) : resolve(repositoryRoot, sourceArgument))
  : resolve(repositoryRoot, 'dist', 'web');
const requested = process.argv[2]?.trim();
const target = requested
  ? (isAbsolute(requested) ? resolve(requested) : resolve(repositoryRoot, requested))
  : resolve(repositoryRoot, 'target', 'release', 'web');

function fail(message: string): never {
  throw new Error(`sync-web-assets failed: ${message}`);
}

async function isFile(path: string): Promise<boolean> {
  return stat(path).then((info) => info.isFile(), () => false);
}

if (!(await isFile(join(source, 'index.html')))) {
  fail(`${source}/index.html is missing; run \`bun scripts/task.ts frontend-build\` first`);
}

// Never delete the source or one of its parents. The default target lives
// under target/, but an arbitrary argument must not be able to wipe dist/.
const sourceWithinTarget = relative(target, source);
if (
  sourceWithinTarget === '' ||
  (!sourceWithinTarget.startsWith('..') && !isAbsolute(sourceWithinTarget))
) {
  fail(`refusing to replace ${target}: it contains the frontend build at ${source}`);
}

await rm(target, { recursive: true, force: true });
await mkdir(dirname(target), { recursive: true });
await cp(source, target, { recursive: true });

console.log(`Synced ${source} -> ${target}`);
