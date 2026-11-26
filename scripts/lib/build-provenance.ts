import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { siteConfig } from './site-config.ts';
export function buildProvenance(kind: 'docs' | 'demo') {
  const root = resolve(import.meta.dirname, '../..');
  const git = (...args: string[]) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
  // docs/experience/releases.json is refreshed from the GitHub API during the
  // site workflow itself: its in-workflow mutation is the intended freshness
  // mechanism, not source drift, so it never marks an artifact dirty.
  const dirty = git('status', '--porcelain', '--', '.', ':(exclude)docs/experience/releases.json') !== '';
  return { version: 1, kind, sourceCommit: git('rev-parse', 'HEAD'), dirty, base: siteConfig().base };
}
