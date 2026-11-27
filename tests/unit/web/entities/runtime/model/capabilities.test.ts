import { describe, expect, it } from 'vitest';
import { createCapabilityLedger } from '../../../../../../src/web/entities/runtime/model/capabilities.ts';
import { RPC } from '../../../../../shared/index.ts';

const methods = (...names: string[]) => ({ methods: names.map((name) => ({ name })) });

describe('capability ledger', () => {
  it('is unknown until a current discovery is accepted', () => {
    const ledger = createCapabilityLedger();
    expect(ledger.supports(RPC.cloudStatus)).toBeUndefined();
    const ticket = ledger.begin('host-a');
    expect(ledger.supports(RPC.cloudStatus)).toBeUndefined();
    expect(ledger.accept(ticket, 'host-a', methods(RPC.cloudStatus))).toBe(true);
    expect(ledger.supports(RPC.cloudStatus)).toBe(true);
    expect(ledger.supports(RPC.syncStatus)).toBe(false);
  });

  it('never lets an older reply replace a newer discovery', () => {
    const ledger = createCapabilityLedger();
    const first = ledger.begin('host-a');
    const second = ledger.begin('host-a');
    expect(ledger.accept(second, 'host-a', methods(RPC.syncStatus))).toBe(true);
    expect(ledger.accept(first, 'host-a', methods(RPC.cloudStatus))).toBe(false);
    expect(ledger.supports(RPC.cloudStatus)).toBe(false);
    expect(ledger.supports(RPC.syncStatus)).toBe(true);
  });

  it('rejects a reply that arrives after the context changed', () => {
    const ledger = createCapabilityLedger();
    const ticket = ledger.begin('host-a');
    ledger.begin('host-b');
    expect(ledger.accept(ticket, 'host-a', methods(RPC.cloudStatus))).toBe(false);
    expect(ledger.snapshot()).toBeNull();
  });

  it('drops the snapshot and rejects replies that were in flight', () => {
    const ledger = createCapabilityLedger();
    const ticket = ledger.begin('host-a');
    ledger.accept(ticket, 'host-a', methods(RPC.cloudStatus));
    const pending = ledger.begin('host-a');
    ledger.invalidate();
    expect(ledger.supports(RPC.cloudStatus)).toBeUndefined();
    expect(ledger.accept(pending, 'host-a', methods(RPC.cloudStatus))).toBe(false);
  });

  it('ignores malformed entries instead of trusting them', () => {
    const ledger = createCapabilityLedger();
    const ticket = ledger.begin('host-a');
    const malformed = { methods: [{ name: RPC.cloudStatus }, { name: '' }, null, { description: 'no name' }] } as unknown as { methods: { name: string }[] };
    ledger.accept(ticket, 'host-a', malformed);
    expect([...ledger.snapshot()!.methods]).toEqual([RPC.cloudStatus]);
  });
});
