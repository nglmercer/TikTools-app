import { expect, test } from 'bun:test';
import { renderControlReference, type ReferenceMethod } from './control-reference.ts';

test('RPC summary links to complete contract metadata and schemas', () => {
  const method: ReferenceMethod = {
    name: 'points.adjust', description: 'Adjust points', sideEffect: true,
    destructive: false, requiresDesktop: false, concurrency: 'additive', durableCommands: true,
    paramsSchema: { type: 'object', required: ['uniqueId', 'delta'] }, resultSchema: { type: 'number' },
  };
  const text = renderControlReference([method], 'es');
  expect(text).toContain('[`points.adjust`](#rpc-points-adjust)');
  expect(text).toContain('### points.adjust {#rpc-points-adjust}');
  expect(text).toContain('"durableCommands":true');
  expect(text).toContain('"concurrency":"additive"');
  expect(text).toContain(JSON.stringify(method.paramsSchema, null, 2));
  expect(text).toContain(JSON.stringify(method.resultSchema, null, 2));
  expect(text).toContain(method.description);
});

test('anchors stay slugify-stable for underscore and camelCase method names', () => {
  const methods: ReferenceMethod[] = ['automation.set_enablement', 'widgets.copyObsUrl', 'cloudLibrary.list'].map((name) => ({
    name, description: 'x', sideEffect: false, destructive: false, requiresDesktop: false,
    concurrency: null, durableCommands: false, paramsSchema: {}, resultSchema: {},
  }));
  const text = renderControlReference(methods, 'en');
  expect(text).toContain('[`automation.set_enablement`](#rpc-automation-set-enablement)');
  expect(text).toContain('### automation.set_enablement {#rpc-automation-set-enablement}');
  expect(text).toContain('[`widgets.copyObsUrl`](#rpc-widgets-copy-obs-url)');
  expect(text).toContain('### cloudLibrary.list {#rpc-cloud-library-list}');
  expect(text).not.toMatch(/#rpc-[a-z0-9-]*[_A-Z]/);
});
