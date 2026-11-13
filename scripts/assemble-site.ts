import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { cp, mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { resolve, relative } from 'node:path';
import { siteConfig } from './lib/site-config.ts';

const root = resolve(import.meta.dirname, '..');
const output = resolve(root, 'dist/site');
const git = (...args: string[]) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
const sourceCommit = git('rev-parse', 'HEAD');
const dirty = git('status', '--porcelain') !== '';
if (dirty && process.env.TIKTOOLS_SITE_ALLOW_DIRTY !== '1') throw new Error('Commit the site source before assembling an artifact, or set TIKTOOLS_SITE_ALLOW_DIRTY=1 for a local preview.');
for (const path of ['dist/docs/index.html', 'dist/demo/index.html']) await readFile(resolve(root, path));
await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });
await cp(resolve(root, 'dist/docs'), output, { recursive: true });
await cp(resolve(root, 'dist/demo'), resolve(output, 'demo'), { recursive: true });
async function files(directory: string): Promise<string[]> {
  const entries = await readdir(directory, { withFileTypes: true });
  return (await Promise.all(entries.map((entry) => entry.isDirectory() ? files(resolve(directory, entry.name)) : [resolve(directory, entry.name)]))).flat();
}
const manifest: Record<string, string> = {};
for (const path of (await files(output)).sort()) manifest[relative(output, path).replaceAll('\\', '/')] = createHash('sha256').update(await readFile(path)).digest('hex');
await writeFile(resolve(output, 'site-manifest.json'), `${JSON.stringify({ version: 1, sourceCommit, dirty, base: siteConfig().base, files: manifest }, null, 2)}\n`);
console.log(`Assembled ${Object.keys(manifest).length} files from ${sourceCommit}${dirty ? ' (local dirty preview)' : ''} into dist/site.`);
