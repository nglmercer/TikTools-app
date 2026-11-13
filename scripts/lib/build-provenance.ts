import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { siteConfig } from './site-config.ts';
export function buildProvenance(kind: 'docs' | 'demo') {
  const root = resolve(import.meta.dirname, '../..');
  const git = (...args: string[]) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
  return { version: 1, kind, sourceCommit: git('rev-parse', 'HEAD'), dirty: git('status', '--porcelain') !== '', base: siteConfig().base };
}
