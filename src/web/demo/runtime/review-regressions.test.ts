import { afterEach, describe, expect, it } from 'vitest';
import type { BehaviorSnapshot, LiveAction } from '../../../automation/behavior/types.ts';
import { callWithMeta, createControlClient } from '@/shared/api/control-client.ts';
import { createDemoHost } from './host.ts';
import { createDemoTransport, demoMethod, objectParams } from './transport.ts';

const MiB = 1024 * 1024;
const cleanup: Array<() => void> = [];
afterEach(() => { for (const dispose of cleanup.splice(0).reverse()) dispose(); });

function hostHarness() {
  const host = createDemoHost('automation');
  const client = createControlClient({ transport: host.transport, timeoutMs: 1000 });
  client.attach();
  cleanup.push(() => { client.detach(); host.dispose(); });
  return { host, client };
}

function request(id: number, method: string) {
  return JSON.stringify({ jsonrpc: '2.0', id, method, params: {} });
}

type WireResponse = {
  response: { id: number; result?: unknown; error?: { code: string } };
};

describe('review regressions: demo runtime contract and isolation', () => {
  for (const method of ['plugins.settings.get', 'plugins.settings.set']) {
    it.each(['constructor', '__proto__'])(`${method} rejects inherited plugin key %s without changing built-ins`, async (pluginId) => {
      const { client, host } = hostHarness();
      const originalValues = Object.getOwnPropertyDescriptor(Object, 'values')!;
      const originalPrototypeValues = Object.getOwnPropertyDescriptor(Object.prototype, 'values');
      let failure: unknown;
      let observedValues: PropertyDescriptor | undefined;
      let observedPrototypeValues: PropertyDescriptor | undefined;
      try {
        try {
          await client.call(method, { pluginId, values: { volume: 5, enabled: true } });
        } catch (error) { failure = error; }
        observedValues = Object.getOwnPropertyDescriptor(Object, 'values');
        observedPrototypeValues = Object.getOwnPropertyDescriptor(Object.prototype, 'values');
      } finally {
        // Restore before assertions: a regression must not poison Vitest or subsequent tests.
        Object.defineProperty(Object, 'values', originalValues);
        if (originalPrototypeValues) Object.defineProperty(Object.prototype, 'values', originalPrototypeValues);
        else Reflect.deleteProperty(Object.prototype, 'values');
      }
      expect(failure).toBeDefined();
      expect(['not_found', 'invalid_params']).toContain((failure as { code?: string } | undefined)?.code);
      expect(observedValues).toEqual(originalValues);
      expect(observedPrototypeValues).toEqual(originalPrototypeValues);
      expect(host.wasSuccessful(method)).toBe(false);
      expect(await client.call('plugins.settings.get', { pluginId: 'demo.audio' })).toMatchObject({
        pluginId: 'demo.audio', values: { volume: 50, enabled: true },
      });
    });
  }

  it('counts committed points adjustments, never failed attempts, as task success', async () => {
    const { host, client } = hostHarness();
    const before = structuredClone(host.state.leaderboard);
    expect(host.wasSuccessful('points.adjust')).toBe(false);
    await expect(client.call('points.adjust', { uniqueId: 'missing-fictional-viewer', delta: 5 }))
      .rejects.toMatchObject({ code: 'not_found' });
    expect(host.state.leaderboard).toEqual(before);
    expect(host.wasSuccessful('points.adjust')).toBe(false);

    const viewer = before[0]!;
    await client.call('points.adjust', { uniqueId: viewer.uniqueId, delta: 5 });
    expect(host.state.leaderboard[0]!.points).toBe(Number(viewer.points) + 5);
    expect(host.wasSuccessful('points.adjust')).toBe(true);

    await expect(client.call('points.adjust', { uniqueId: 'still-missing', delta: 5 }))
      .rejects.toMatchObject({ code: 'not_found' });
    expect(host.wasSuccessful('points.adjust')).toBe(true);
  });

  it('records accepted automation writes through the same success API as ordinary host methods', async () => {
    const { host, client } = hostHarness();
    const record: LiveAction = {
      schemaVersion: 2, id: 'review-regression-log', name: 'Reviewed demo log',
      typeId: 'core.log', enabled: false, config: { message: 'Fictional example' },
    };
    expect(host.wasSuccessful('automation.create')).toBe(false);
    await expect(client.call('automation.create', { kind: 'action', record: { id: record.id, name: record.name } }))
      .rejects.toMatchObject({ code: 'invalid_params' });
    expect(host.wasSuccessful('automation.create')).toBe(false);
    await client.call('automation.create', { kind: 'action', record }, { meta: { expectedRevision: '0' } });
    expect(host.wasSuccessful('automation.create')).toBe(true);
    expect(await client.call('automation.get', { kind: 'action', id: record.id })).toEqual(record);
  });

  it('rejects a UTF-8 reply above 2 MiB instead of enqueuing an oversized success', async () => {
    // Below 2 MiB in JS string units, above it in UTF-8: character counts are insufficient.
    const payload = 'é'.repeat(MiB + 1);
    const peer = createDemoTransport({
      'fixture.large': demoMethod('Read oversized fictional data', objectParams, () => ({ result: payload })),
    });
    cleanup.push(() => peer.transport.detach());
    const delivered: WireResponse[] = [];
    peer.transport.attach((raw) => {
      expect(new TextEncoder().encode(raw).byteLength).toBeLessThanOrEqual(2 * MiB);
      delivered.push(JSON.parse(raw) as WireResponse);
    }, () => {}, () => {});
    expect(peer.diagnostics()).toMatchObject({ maxMessageBytes: 2 * MiB, maxQueueBytes: 4 * MiB });
    peer.transport.send(request(1, 'fixture.large'));
    await Promise.resolve();
    expect(delivered).toHaveLength(1);
    expect(delivered[0]!.response).toMatchObject({ id: 1, error: { code: 'too_large' } });
    expect(delivered[0]!.response).not.toHaveProperty('result');
    expect(peer.diagnostics().queuedBytes).toBe(0);
  });

  it('reserves a full reply before executing a mutation and releases byte capacity after delivery', async () => {
    let mutations = 0;
    const peer = createDemoTransport({
      'fixture.read': demoMethod('Read bounded fictional data', objectParams, () => ({ result: 'é'.repeat(700 * 1024) })),
      'fixture.mutate': demoMethod('Mutate isolated test state', objectParams, () => { mutations++; return { result: null }; }, true),
    });
    cleanup.push(() => peer.transport.detach());
    // Leave unattached so the first two replies remain queued deterministically.
    peer.transport.send(request(1, 'fixture.read'));
    peer.transport.send(request(2, 'fixture.read'));
    const queued = peer.diagnostics();
    expect(queued).toMatchObject({ queuedResponses: 2, maxMessageBytes: 2 * MiB, maxQueueBytes: 4 * MiB });
    expect(queued.queuedBytes).toBeGreaterThan(2 * MiB);
    expect(queued.queuedBytes).toBeLessThanOrEqual(4 * MiB);
    let admissionFailure: unknown;
    try { peer.transport.send(request(3, 'fixture.mutate')); } catch (error) { admissionFailure = error; }
    expect(mutations).toBe(0);
    expect(peer.diagnostics().queuedBytes).toBeLessThanOrEqual(4 * MiB);

    const delivered: WireResponse[] = [];
    peer.transport.attach((raw) => delivered.push(JSON.parse(raw) as WireResponse), () => {}, () => {});
    await Promise.resolve();
    // Either a synchronous send rejection or a correlated overload reply is a valid transport boundary.
    if (admissionFailure) expect(admissionFailure).toMatchObject({ code: 'overloaded' });
    else expect(delivered.find((item) => item.response.id === 3)?.response.error?.code).toBe('overloaded');
    expect(peer.diagnostics()).toMatchObject({ queuedResponses: 0, queuedBytes: 0 });

    peer.transport.send(request(4, 'fixture.mutate'));
    await Promise.resolve();
    expect(mutations).toBe(1);
    expect(delivered.find((item) => item.response.id === 4)?.response).toEqual({ jsonrpc: '2.0', id: 4, result: null });
    expect(peer.diagnostics()).toMatchObject({ queuedResponses: 0, queuedBytes: 0 });
  });

  it('clears queued bytes and invalidates delivery on disconnect and detach', async () => {
    const peer = createDemoTransport({
      'fixture.read': demoMethod('Read fictional data', objectParams, () => ({ result: 'x'.repeat(1024) })),
    });
    cleanup.push(() => peer.transport.detach());
    peer.transport.send(request(1, 'fixture.read'));
    expect(peer.diagnostics().queuedBytes).toBeGreaterThan(0);
    peer.disconnect();
    expect(peer.diagnostics()).toMatchObject({ queuedResponses: 0, queuedBytes: 0 });
    peer.reconnect();
    peer.transport.send(request(2, 'fixture.read'));
    const delivered: string[] = [];
    peer.transport.attach((raw) => delivered.push(raw), () => {}, () => {});
    peer.transport.detach();
    await Promise.resolve();
    expect(delivered).toEqual([]);
    expect(peer.diagnostics()).toMatchObject({ queuedResponses: 0, queuedBytes: 0, callbacks: 0 });
  });

  it('rejects an update identity mismatch without changing the original record, references or revisions', async () => {
    const { host, client } = hostHarness();
    const before = await callWithMeta<BehaviorSnapshot>(client, 'automation.snapshot', {});
    const original = before.result.actions[0]!;
    const version = before.meta!.resourceVersions.find((item) => item.scope === 'action' && item.id === original.id)!;
    const candidate = { ...original, id: 'unexpected-renamed-identity', name: 'Must not be committed' };
    await expect(client.call('automation.update', { kind: 'action', id: original.id, record: candidate }, {
      meta: { expectedRevision: version.revision },
    })).rejects.toMatchObject({ code: 'invalid_params' });
    expect(host.wasSuccessful('automation.update')).toBe(false);

    const after = await callWithMeta<BehaviorSnapshot>(client, 'automation.snapshot', {});
    expect(after.result).toEqual(before.result);
    expect(after.meta?.resourceVersions).toEqual(before.meta!.resourceVersions);
    const retained = await callWithMeta<LiveAction>(client, 'automation.get', { kind: 'action', id: original.id });
    expect(retained.result).toEqual(original);
    expect(retained.meta?.resourceVersions).toEqual([version]);

    // The rejected attempt did not consume the original CAS revision.
    const valid = { ...original, name: 'Accepted update' };
    const saved = await callWithMeta<LiveAction>(client, 'automation.update', { kind: 'action', id: original.id, record: valid }, {
      meta: { expectedRevision: version.revision },
    });
    expect(saved.result).toEqual(valid);
    expect(saved.meta?.resourceVersions).toEqual([{ ...version, revision: String(BigInt(version.revision) + 1n) }]);
    expect(host.wasSuccessful('automation.update')).toBe(true);
  });
});
