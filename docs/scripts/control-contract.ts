import { execFileSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
const root = resolve(import.meta.dirname, '../..');
export const digest = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex');
export async function contractSources(): Promise<Record<string, string>> {
  // Registry schemas also embed core/plugin/remote types. Conservatively track
  // Rust source changes rather than silently shipping stale generated tables.
  const paths = execFileSync('git', ['ls-files', 'crates', 'Cargo.toml', 'Cargo.lock'], { cwd: root, encoding: 'utf8' }).trim().split('\n').filter((path) => path.endsWith('.rs') || path.endsWith('Cargo.toml') || path === 'Cargo.lock');
  return Object.fromEntries(await Promise.all(paths.sort().map(async (path) => [path, digest(await readFile(resolve(root, path)))])));
}
export async function checkControlContract(): Promise<void> {
  const provenance = JSON.parse(await readFile(resolve(root, 'docs/contracts/provenance.json'), 'utf8')) as { schemaSha256: string; cliHelpSha256?: string; sources: Record<string, string> };
  const staleCliHelp = provenance.cliHelpSha256 !== digest(await readFile(resolve(root, 'docs/contracts/cli-help.json')));
  if (provenance.schemaSha256 !== digest(await readFile(resolve(root, 'docs/contracts/control-api.json'))) || staleCliHelp || JSON.stringify(provenance.sources) !== JSON.stringify(await contractSources())) throw new Error('The versioned RPC/CLI reference is stale. Regenerate with bun docs/scripts/regenerate-control.ts after changing Rust sources. Normal docs builds do not run Cargo.');
}
