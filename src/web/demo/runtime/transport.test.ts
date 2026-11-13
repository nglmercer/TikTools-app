import { describe, expect, it } from 'vitest';
import { callWithMeta, createControlClient } from '@/shared/api/control-client.ts';
import { createDemoTransport, demoMethod, objectParams } from './transport.ts';

describe('demo ControlTransport protocol', () => {
  it('answers the client ID in the real rpc-response envelope with typed metadata', async () => {
    const peer = createDemoTransport({ 'sample.read': demoMethod('Read fixture', objectParams, () => ({ result: { value: 7 }, meta: { resourceVersions: [{ kind: 'sample', id: 'one', scope: 'demo', revision: '1' }] } })) });
    const client = createControlClient({ transport: peer.transport, timeoutMs: 1000 });
    client.attach();
    expect(await callWithMeta(client, 'sample.read', {})).toEqual({ result: { value: 7 }, meta: { resourceVersions: [{ kind: 'sample', id: 'one', scope: 'demo', revision: '1' }] } });
    expect(client.diagnostics?.().pending).toBe(0);
    client.detach();
  });
  it('discovers exactly the registered methods and rejects unsupported effects immediately', async () => {
    const peer = createDemoTransport({ 'sample.read': demoMethod('Read fixture', objectParams, () => ({ result: null })) });
    const client = createControlClient({ transport: peer.transport, timeoutMs: 1000 });
    client.attach();
    const discovery = await client.call<{ methods: Array<{ name: string }> }>('rpc.discover');
    expect(discovery.methods.map((method) => method.name).sort()).toEqual(['rpc.discover', 'sample.read']);
    await expect(client.call('shell.run', { command: 'anything' })).rejects.toMatchObject({ code: 'method_not_found' });
    await expect(client.call('__proto__')).rejects.toMatchObject({ code: 'method_not_found' });
    expect(client.diagnostics?.().pending).toBe(0);
    client.detach();
  });
  it('queues a bootstrap response until attached and clears callbacks and pending calls at detach', async () => {
    const peer = createDemoTransport({ 'sample.read': demoMethod('Read fixture', objectParams, () => ({ result: true })) });
    const client = createControlClient({ transport: peer.transport, timeoutMs: 1000 });
    const waiting = client.call('sample.read');
    expect(peer.diagnostics().queuedResponses).toBe(1);
    client.attach();
    expect(await waiting).toBe(true);
    const cancelled = client.call('sample.read');
    client.detach();
    await expect(cancelled).rejects.toMatchObject({ code: 'transport' });
    await Promise.resolve();
    expect(peer.diagnostics()).toMatchObject({ queuedResponses: 0, callbacks: 0, detached: true });
    expect(client.diagnostics?.().pending).toBe(0);
  });
  it('delivers events to the existing client topic subscription without a native IPC global', async () => {
    const peer = createDemoTransport({});
    const client = createControlClient({ transport: peer.transport });
    client.attach();
    const values: unknown[] = [];
    const stop = client.onTopic('demo.event', (value) => values.push(value));
    peer.emitTopic('demo.event', { count: 1 });
    await Promise.resolve();
    expect(values).toEqual([{ count: 1 }]);
    stop();
    client.detach();
  });
});
