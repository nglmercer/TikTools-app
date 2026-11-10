import { describe, expect, it } from 'vitest';
import { parseGrant } from './grants.ts';
import { viewerGrant } from '../api/types.ts';

describe('automation resource restrictions', () => {
  it('requires an explicit family without widening malformed or old grants', () => {
    const grant = () => ({ ...viewerGrant(), resources: { automation: ['event:same', 'action:same', 'event:with:colon'] } });
    expect(parseGrant(grant()).resources.automation).toEqual(['event:same', 'action:same', 'event:with:colon']);
    expect(parseGrant({ ...viewerGrant(), resources: { automation: [] } }).resources.automation).toEqual([]);
    for (const bad of ['same', 'host:same', 'Event:same', 'event:', 'event: same', 'event:same ', 'event:*', 'event:bad\n', `event:${'á'.repeat(65)}`]) {
      expect(() => parseGrant({ ...grant(), resources: { automation: [bad] } })).toThrow('invalid_grant');
    }
  });
});

describe('reusable automation policy storage', () => {
  const grant = () => ({ ...viewerGrant(), capabilities: ['automation.read', 'automation.edit', 'automation.toggle'],
    automationDelegation: { effects: ['log', 'delay'], allowDynamicTargets: false } });
  it('preserves explicit policy and rejects unknown effects or missing authority', () => {
    expect(parseGrant(grant()).automationDelegation).toEqual({ effects: ['log', 'delay'], allowDynamicTargets: false });
    expect(parseGrant(viewerGrant()).automationDelegation).toBeUndefined();
    for (const bad of [[], ['plugin'], ['log', 'log'], ['http']]) {
      expect(() => parseGrant({ ...grant(), automationDelegation: { effects: bad, allowDynamicTargets: false } })).toThrow('invalid_grant');
    }
    expect(() => parseGrant({ ...grant(), capabilities: ['automation.read'] })).toThrow('invalid_grant');
    expect(() => parseGrant({ ...grant(), automationDelegation: { effects: ['log'], allowDynamicTargets: false, unknown: true } })).toThrow('invalid_grant');
  });
});
