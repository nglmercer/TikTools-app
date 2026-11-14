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
