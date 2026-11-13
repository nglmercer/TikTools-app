import { appendFileSync, readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

export const fullScope = () => ({ frontend: true, rust: true, msrv: true, desktop: true, pluginRuntime: true });
const emptyScope = () => Object.fromEntries(Object.keys(fullScope()).map((key) => [key, false]));

/** Unknown paths and failed comparisons run every existing gate. Native suites
 * remain complete and serial; only changes proven to be frontend/docs can omit them. */
export function scopeForPaths(paths) {
  const scope = emptyScope();
  for (const path of paths) {
    if (typeof path !== 'string' || !path || path.includes('..') || path.includes('\\')) return fullScope();
    if (path.startsWith('docs/examples/')) { scope.frontend = true; continue; }
    if (path.startsWith('docs/') || /^(README|CONTRIBUTING|CHANGELOG|LICENSE)(\.[^/]*)?$/.test(path)) continue;
    if (path === '.github/workflows/site.yml') continue; // Explicit manual site gate.
    if (/^(src\/web\/|src\/widgets\/|src\/shared\/|src\/plugin-ui\/|tests\/(site|e2e)\/)/.test(path)
        || /^(vite(?:\.[\w-]+)?\.config\.ts|vitest(?:\.[\w-]+)?\.config\.ts|playwright(?:\.[\w-]+)?\.config\.ts|tsconfig(?:\.[\w-]+)?\.json|eslint\.config\.[cm]?js)$/.test(path)) {
      scope.frontend = true;
      continue;
    }
    return fullScope();
  }
  return scope;
}

export function eventComparison(eventName, event) {
  if (eventName === 'pull_request') return { base: event.pull_request?.base?.sha, head: event.pull_request?.head?.sha };
  if (eventName === 'push') return { base: event.before, head: event.after };
  return null;
}

export function selectScope({ eventName, forceFull = false, suite, changedPaths }) {
  if (forceFull || suite === 'full') return fullScope();
  if (eventName === 'workflow_dispatch') {
    if (suite === 'frontend') return { ...emptyScope(), frontend: true };
    if (suite === 'native') return { ...fullScope(), frontend: false };
    return fullScope();
  }
  return Array.isArray(changedPaths) ? scopeForPaths(changedPaths) : fullScope();
}

function comparePaths(comparison) {
  const sha = /^[a-f0-9]{40}$/;
  if (!comparison || !sha.test(comparison.base ?? '') || !sha.test(comparison.head ?? '')
      || /^0+$/.test(comparison.base)) return undefined;
  for (const revision of [comparison.base, comparison.head]) {
    if (spawnSync('git', ['cat-file', '-e', `${revision}^{commit}`]).status !== 0) {
      // Checkout is shallow. Fetch only a missing comparison commit, never the
      // whole history. Failure falls back to the complete gate set.
      if (spawnSync('git', ['fetch', '--no-tags', '--depth=1', 'origin', revision], { timeout: 30_000, stdio: 'ignore' }).status !== 0) return undefined;
    }
  }
  const result = spawnSync('git', ['diff', '--name-only', '--no-renames', '-z', comparison.base, comparison.head], { encoding: 'utf8', timeout: 30_000, maxBuffer: 16 * 1024 * 1024 });
  return result.status === 0 ? result.stdout.split('\0').filter(Boolean) : undefined;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const eventName = process.env.GITHUB_EVENT_NAME;
  const event = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, 'utf8'));
  const changedPaths = comparePaths(eventComparison(eventName, event));
  const scope = selectScope({ eventName, forceFull: process.env.CI_FORCE_FULL === 'true', suite: process.env.CI_SUITE, changedPaths });
  const output = Object.entries(scope).map(([key, value]) => `${key}=${value}`).join('\n') + '\n';
  if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, output);
  console.log(output.trim());
  console.log(changedPaths ? `Compared ${changedPaths.length} changed paths.` : 'No Git comparison; applied the explicit manual/reusable policy or full fallback.');
}
