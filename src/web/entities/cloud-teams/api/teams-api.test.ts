import { describe, expect, it } from 'vitest';
import { InvalidTeamResponse, parseGrants, parseHost, parseIssuedInvitation } from './teams-api.ts';

const id = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';

describe('team response parsing', () => {
  it('maps a team and defaults online to false for creation replies', () => {
    expect(parseHost({ id, name: 'Stream', status: 'active', protocolVersion: 1, revision: 1 }))
      .toEqual({ id, name: 'Stream', status: 'active', protocolVersion: 1, revision: 1, online: false });
  });

  it('refuses identifiers that are not UUIDs before they reach a path', () => {
    expect(() => parseHost({ id: '../x', name: 'a', status: 'active', protocolVersion: 1, revision: 1 }))
      .toThrow(InvalidTeamResponse);
  });

  it('requires a token of realistic length for an invitation', () => {
    const issued = parseIssuedInvitation({ id, requestedRole: 'observer', expiresAt: 'later', token: 'x'.repeat(43) });
    expect(issued.token).toHaveLength(43);
    expect(() => parseIssuedInvitation({ id, requestedRole: 'observer', expiresAt: 'later', token: 'short' }))
      .toThrow(InvalidTeamResponse);
  });

  it('keeps the revision a role change must echo back', () => {
    const [grant] = parseGrants({ items: [{ id, deviceId: id, deviceLabel: 'PC', role: 'observer', status: 'active', revision: 4, policy: {} }] });
    expect(grant?.revision).toBe(4);
  });
});
