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
