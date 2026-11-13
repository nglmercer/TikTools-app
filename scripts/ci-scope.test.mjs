import { test } from 'node:test';
import assert from 'node:assert/strict';
import { eventComparison, fullScope, scopeForPaths, selectScope } from './ci-scope.mjs';

test('documentation changes do not compile native applications', () => {
  assert.ok(Object.values(scopeForPaths(['docs/content/es/user/live/feed.md', 'docs/package.json', 'README.md'])).every((value) => value === false));
});
test('real Vue, demo and browser tests keep the frontend gate', () => {
  for (const path of ['src/web/app/main.ts', 'src/web/demo/main.ts', 'docs/examples/ButtonsExample.vue', 'tests/site/demo.spec.ts', 'vitest.config.ts', 'vite.demo.config.ts']) {
    const scope = scopeForPaths([path]); assert.equal(scope.frontend, true); assert.equal(scope.rust, false);
  }
});
test('native sources, build scripts, lockfiles and unknown paths preserve every gate', () => {
  for (const path of ['crates/tiktools-core/src/lib.rs', 'Cargo.toml', 'Cargo.lock', 'bun.lock', 'package.json', 'scripts/task.ts', '.github/workflows/ci.yml', 'new-build-system.toml']) assert.deepEqual(scopeForPaths([path]), fullScope());
});
test('mixed changes, deletions and renames cannot hide a native change', () => {
  assert.deepEqual(scopeForPaths(['docs/content/en/index.md', 'src/web/App.vue', 'crates/removed.rs']), fullScope());
  assert.deepEqual(scopeForPaths(['../bad', 'docs/index.md']), fullScope());
});
test('manual full/native/frontend modes and unreliable comparisons are explicit', () => {
  assert.deepEqual(selectScope({ eventName: 'push' }), fullScope());
  assert.deepEqual(selectScope({ eventName: 'workflow_dispatch', suite: 'full', changedPaths: [] }), fullScope());
  assert.equal(selectScope({ eventName: 'workflow_dispatch', suite: 'native' }).rust, true);
  assert.equal(selectScope({ eventName: 'workflow_dispatch', suite: 'frontend' }).rust, false);
  assert.deepEqual(selectScope({ eventName: 'push', forceFull: true, changedPaths: ['docs/index.md'] }), fullScope());
});
test('comparison uses both PR endpoints and the complete push range', () => {
  assert.deepEqual(eventComparison('pull_request', { pull_request: { base: { sha: 'base' }, head: { sha: 'head' } } }), { base: 'base', head: 'head' });
  assert.deepEqual(eventComparison('push', { before: 'before', after: 'after' }), { base: 'before', head: 'after' });
  assert.equal(eventComparison('workflow_dispatch', {}), null);
});
