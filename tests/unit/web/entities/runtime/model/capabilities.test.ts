import { describe, expect, it } from 'vitest';
import { createCapabilityLedger, type DiscoverResult } from '../../../../../../src/web/entities/runtime/model/capabilities.ts';
import { FIXTURE_ID, RPC, defined } from '../../../../../shared/index.ts';

const methods = (...names: string[]) => ({ methods: names.map((name) => ({ name })) });

describe('capability ledger', () => {
  it('is unknown until a current discovery is accepted', () => {
    const ledger = createCapabilityLedger();
    expect(ledger.supports(RPC.cloudStatus)).toBeUndefined();
    const ticket = ledger.begin(FIXTURE_ID.hostA);
    expect(ledger.supports(RPC.cloudStatus)).toBeUndefined();
    expect(ledger.accept(ticket, FIXTURE_ID.hostA, methods(RPC.cloudStatus))).toBe(true);
    expect(ledger.supports(RPC.cloudStatus)).toBe(true);
    expect(ledger.supports(RPC.syncStatus)).toBe(false);
  });

  it('never lets an older reply replace a newer discovery', () => {
    const ledger = createCapabilityLedger();
    const first = ledger.begin(FIXTURE_ID.hostA);
    const second = ledger.begin(FIXTURE_ID.hostA);
    expect(ledger.accept(second, FIXTURE_ID.hostA, methods(RPC.syncStatus))).toBe(true);
    expect(ledger.accept(first, FIXTURE_ID.hostA, methods(RPC.cloudStatus))).toBe(false);
    expect(ledger.supports(RPC.cloudStatus)).toBe(false);
    expect(ledger.supports(RPC.syncStatus)).toBe(true);
  });

  it('rejects a reply that arrives after the context changed', () => {
    const ledger = createCapabilityLedger();
    const ticket = ledger.begin(FIXTURE_ID.hostA);
    ledger.begin('host-b');
    expect(ledger.accept(ticket, FIXTURE_ID.hostA, methods(RPC.cloudStatus))).toBe(false);
    expect(ledger.snapshot()).toBeNull();
  });

  it('drops the snapshot and rejects replies that were in flight', () => {
    const ledger = createCapabilityLedger();
    const ticket = ledger.begin(FIXTURE_ID.hostA);
    ledger.accept(ticket, FIXTURE_ID.hostA, methods(RPC.cloudStatus));
    const pending = ledger.begin(FIXTURE_ID.hostA);
    ledger.invalidate();
    expect(ledger.supports(RPC.cloudStatus)).toBeUndefined();
    expect(ledger.accept(pending, FIXTURE_ID.hostA, methods(RPC.cloudStatus))).toBe(false);
  });

  it('ignores malformed entries instead of trusting them', () => {
    const ledger = createCapabilityLedger();
    const ticket = ledger.begin(FIXTURE_ID.hostA);
    // A reply as it arrives off the wire: JSON.parse yields untyped data, so no assertion is needed to hold entries the type forbids
    const malformed: DiscoverResult = JSON.parse(
      JSON.stringify({ methods: [{ name: RPC.cloudStatus }, { name: '' }, null, { description: 'no name' }] }),
    );
    ledger.accept(ticket, FIXTURE_ID.hostA, malformed);
    expect([...defined(ledger.snapshot()).methods]).toEqual([RPC.cloudStatus]);
  });
});
