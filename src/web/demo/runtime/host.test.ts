import { describe, expect, it } from 'vitest';
import { createControlClient } from '@/shared/api/control-client.ts';
import { createDemoHost } from './host.ts';
import { parseDemoOptions, scenarios } from '../scenarios.ts';
import { createLiveFeedState } from '@/entities/live-event/model/live-feed-state.ts';
import { flushPromises } from '@vue/test-utils';

describe('public demo host', () => {
  it('feeds the real application state through its current topic and journal contract', async () => {
    const host = createDemoHost('feed'); const client = createControlClient({ transport: host.transport }); client.attach();
    const feed = createLiveFeedState(client, { systemAuthor: () => 'Demo', discontinuityText: () => 'Resumed' });
    host.step();
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 120));
    expect(feed.events.value.some((event) => event.text === 'Hello TikTools!')).toBe(true);
    expect(feed.diagnostics()).toMatchObject({ received: 1, cursor: 1 });
    feed.dispose(); client.detach(); host.dispose();
  });
  it('connects, delivers an event and records a simulated automation result', async () => {
    const host = createDemoHost('connection'); const client = createControlClient({ transport: host.transport }); client.attach();
    const feed: unknown[] = []; const runs: unknown[] = [];
    client.onTopic('live.ui-event', (event) => feed.push(event)); client.onTopic('automation.run.completed', (run) => runs.push(run));
    expect(host.step()).toBe(false);
    await client.call('live.connect', { uniqueId: 'fictional_creator' });
    expect(host.step()).toBe(true);
    await Promise.resolve();
    expect(feed).toHaveLength(1); expect(runs).toHaveLength(1);
    expect(host.state.automationRuns[0]).toMatchObject({ summary: 'Hello Luna', test: false, status: 'ok' });
    for (let i = 0; i < 400; i++) { host.step(); await Promise.resolve(); }
    expect(host.diagnostics()).toMatchObject({ ticks: 300, events: 60, runs: 60 });
    client.detach(); host.dispose();
    expect(host.diagnostics()).toMatchObject({ callbacks: 0, queuedResponses: 0, disposed: true, events: 0, runs: 0 });
  });
  it('rejects stale changes and never executes supplied code or network effects', async () => {
    const host = createDemoHost('cloud'); const client = createControlClient({ transport: host.transport }); client.attach();
    const call = () => client.call('sync.v2.variables.apply', { review: 'demo-review-1' });
    await call(); await expect(call()).rejects.toMatchObject({ code: 'conflict' });
    await expect(client.call('automation.script.analyze', { source: 'throw new Error()' })).rejects.toMatchObject({ code: 'demo_unsupported' });
    await expect(client.call('plugins.action.execute', { actionType: 'http', config: { url: 'https://example.com' } })).rejects.toMatchObject({ code: 'demo_unsupported' });
    await expect(client.call('shell.execute', {})).rejects.toMatchObject({ code: 'method_not_found' });
    client.detach(); host.dispose();
  });
  it('has deterministic initial state and bounded explicit startup replies for every scenario', async () => {
    for (const id of Object.keys(scenarios) as Array<keyof typeof scenarios>) {
      const a = createDemoHost(id); const b = createDemoHost(id);
      expect(a.state).toEqual(b.state);
      const client = createControlClient({ transport: a.transport }); client.attach();
      for (const method of ['system.info', 'app.state.get', 'automation.snapshot', 'automation.runs', 'points.config.get', 'points.leaderboard', 'live.status', 'live.monitor.status', 'creators.recent', 'globals.list', 'processors.status', 'widgets.status']) expect(await client.call(method, {})).toBeDefined();
      expect(client.diagnostics?.().pending).toBe(0); client.detach(); a.dispose(); b.dispose();
    }
  });
  it('validates public parameters without promoting hidden screens', () => {
    expect(parseDemoOptions('?scenario=feed&locale=fr').locale).toBe('fr');
    for (const params of ['?route=//evil.test', '?route=/rankings', '?tour=unknown', '?scenario=unknown', '?instance=../other', '?locale=xx']) expect(() => parseDemoOptions(params)).toThrow();
  });
});
