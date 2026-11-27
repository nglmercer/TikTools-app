import { expect, it } from 'vitest';
import { demoPreviewUrl } from '../../../src/web/demo/preview-url.ts';
import { demoParam, FLAG_ON } from '../../../src/web/demo/contract.ts';

it('builds a same-origin framed preview for the tour scenario and route', () => {
  const value = new URL(demoPreviewUrl('connection', 'en'));
  expect(value.origin).toBe(window.location.origin);
  expect(value.searchParams.get(demoParam.scenario)).toBe('connection');
  expect(value.searchParams.get(demoParam.framed)).toBe(FLAG_ON);
  expect(value.searchParams.get(demoParam.preview)).toBe(FLAG_ON);
  expect(value.searchParams.get(demoParam.route)).toBe('/connections');
  expect(value.searchParams.get(demoParam.instance)).toMatch(/^catalog-connection-[a-z0-9]+/);
  expect(value.hash).toBe('#/connections');
});

it('keeps cloud variable tours on their scenario route and rejects unsupported tours', () => {
  const value = new URL(demoPreviewUrl('changes', 'es'));
  expect(value.searchParams.get(demoParam.scenario)).toBe('cloud');
  expect(value.searchParams.get(demoParam.route)).toBe('/automations?view=variables');
  expect(value.hash).toBe('#/automations?view=variables');
  expect(demoPreviewUrl('backup', 'es')).toBe('');
});
