import { describe, expect, it } from 'vitest';
import { createControlClient } from '@/shared/api/control-client.ts';
import { createDemoHost } from './host.ts';
import { parseDemoOptions, scenarios } from '../scenarios.ts';
import { createLiveFeedState } from '@/entities/live-event/model/live-feed-state.ts';
import { flushPromises } from '@vue/test-utils';
import { parseHosts, parseDevices, parseAccessibleTeams } from '@/entities/cloud-teams/api/teams-api.ts';
import type { AnalyticsTotals, GiftCatalogEntry, UiEvent } from '../../../shared/messages.ts';
import { DEMO_ACTIVITY_LENGTH } from '../fixtures/audience.ts';

describe('public demo host', () => {
  it('populates the feed, profiles, gift catalog and analytics from the same six fictional viewers', async () => {
    const host = createDemoHost('feed', 'es'); const client = createControlClient({ transport: host.transport }); client.attach();
    for (let i = 0; i < DEMO_ACTIVITY_LENGTH; i++) { host.step(); await Promise.resolve(); }
    const events = host.state.feedJournal!.map((row) => row.event as unknown as UiEvent);
    expect(new Set(events.slice(0, 9).map((event) => event.author)).size).toBe(6);
    expect(new Set(events.map((event) => event.kind))).toEqual(new Set(['chat', 'like', 'gift', 'member', 'social']));
    expect(events[6]).toMatchObject({ author: 'milo', text: 'Saludos desde Lima 👋' });
    expect(host.state.automationRuns).toEqual(expect.arrayContaining([expect.objectContaining({ summary: 'Hello Kai', test: false })]));
    const catalog = await client.call<{ gifts: GiftCatalogEntry[] }>('gifts.list', {});
    expect(catalog.gifts[0]).toMatchObject({ id: 'demo-rose', iconUrl: expect.any(String) });
    expect(events.filter((event) => event.kind === 'gift').map((event) => event.giftDetails?.count)).toEqual([1, 3, 2, 5, 1]);
    for (const event of events) {
      expect(host.state.leaderboard.some((viewer) => viewer.uniqueId === event.author)).toBe(true);
      if (event.kind === 'gift') expect(event.giftDetails?.imageUrl).toBe(catalog.gifts[0]!.iconUrl);
    }
    for (const viewer of host.state.leaderboard) {
      const last = events.findLast((event) => event.author === viewer.uniqueId)!;
      expect(viewer).toMatchObject({ points: last.points, level: last.level });
    }
    const { summary } = await client.call<{ summary: { totals: AnalyticsTotals; topViewers: Array<{ chats: number; likes: number; shares: number; diamonds: number }> } }>('analytics.summary', {});
    expect(summary.topViewers).toHaveLength(6);
    for (const [total, key] of [['chats', 'chats'], ['likes', 'likes'], ['shares', 'shares'], ['diamonds', 'diamonds']] as const) expect(summary.totals[total]).toBe(summary.topViewers.reduce((sum, viewer) => sum + viewer[key], 0));
    client.detach(); host.dispose();
  });
  it('awards the actual sender using the edited rates, toggles, subscriber bonus and gift quantity', async () => {
    const host = createDemoHost('feed'); const client = createControlClient({ transport: host.transport }); client.attach();
    const viewer = (id: string) => host.state.leaderboard.find((user) => user.uniqueId === id)!;
    host.step(); await Promise.resolve();
    expect(viewer('luna').points).toBe(121);
    await client.call('points.config.set', { pointsPerLikeEnabled: false, pointsPerCoin: 2, pointsPerChat: 3, subBonusMultiplier: 100 });
    host.step(); await Promise.resolve();
    expect(viewer('milo')).toMatchObject({ points: 85, totalLikes: 42 });
    expect(viewer('luna').points).toBe(121);
    host.step(); await Promise.resolve();
    expect(viewer('sofia')).toMatchObject({ points: 66, totalCoins: 9 });
    host.step(); await Promise.resolve();
    expect(viewer('kai')).toMatchObject({ points: 48, totalChats: 10 });
    for (let i = 0; i < 5; i++) { host.step(); await Promise.resolve(); }
    expect(viewer('nora')).toMatchObject({ points: 34, totalCoins: 5 });
    expect(viewer('leo').points).toBe(20);
    client.detach(); host.dispose();
  });
  it('keeps offline and session examples disconnected, then permits recovery without credentials', async () => {
    const host = createDemoHost('connection'); const client = createControlClient({ transport: host.transport }); client.attach();
    for (const [kind, code] of [['offline', 'creator_offline'], ['auth', 'authentication_required']] as const) {
      host.setConnectionExample(kind, `Example: ${kind}`);
      await expect(client.call('live.connect', { uniqueId: 'demo_creator' })).rejects.toMatchObject({ code, message: `Example: ${kind}` });
      expect(host.state.liveStatus.connected).toBe(false);
      expect(host.step()).toBe(false);
    }
    host.setConnectionExample('normal');
    await client.call('live.connect', { uniqueId: 'demo_creator' });
    expect(host.state.liveStatus.connected).toBe(true);
    expect(host.step()).toBe(true);
    client.detach(); host.dispose();
  });
  it('simulates reversible action errors without claiming that the LIVE connection failed', () => {
    const host = createDemoHost('feed');
    host.simulateError(true);
    host.step();
    expect(host.state.automationRuns.at(-1)).toMatchObject({ status: 'error', error: expect.stringContaining('simulated action failure') });
    expect(host.state.liveStatus.connected).toBe(true);
    host.simulateError(false);
    host.step(); host.step(); host.step();
    expect(host.state.automationRuns.at(-1)).toMatchObject({ status: 'ok' });
    host.dispose();
  });
  it('keeps experimental Teams readable and states the native boundary explicitly', async () => {
    const host = createDemoHost('collaboration'); const client = createControlClient({ transport: host.transport }); client.attach();
    expect(parseHosts(await client.call('teams.hosts.list'))).toEqual([]);
    expect(parseDevices(await client.call('teams.devices.list'))).toEqual([]);
    expect(parseAccessibleTeams(await client.call('teams.accessible.list'))).toEqual([]);
    await expect(client.call('teams.hosts.create', { name: 'Fictional team' })).rejects.toMatchObject({ code: 'demo_unsupported' });
    expect(host.wasSuccessful('teams.hosts.create')).toBe(false);
    client.detach(); host.dispose();
  });
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
    expect(parseDemoOptions('?embed=feed').route).toBe('/feed');
    expect(parseDemoOptions('?framed=1').framed).toBe(true);
    expect(parseDemoOptions('').framed).toBe(false);
    for (const params of ['?framed=0', '?framed=true', '?framed=other']) expect(() => parseDemoOptions(params)).toThrow();
    expect(parseDemoOptions('?embed=feed').initialEvents).toBe(1);
    expect(parseDemoOptions('?embed=feed&events=9').initialEvents).toBe(9);
    for (const params of ['?embed=feed&events=13', '?embed=feed&events=-1', '?embed=feed&events=1.5', '?events=9', '?embed=connection&events=9']) expect(() => parseDemoOptions(params)).toThrow();
    expect(parseDemoOptions('?embed=connection').embed).toBe('connection');
    for (const params of ['?embed=unknown', '?embed=connection&route=/feed', '?embed=connection&tour=connection']) expect(() => parseDemoOptions(params)).toThrow();
    expect(parseDemoOptions('?scenario=feed&locale=fr').locale).toBe('fr');
    expect(parseDemoOptions('?scenario=feed&locale=zh-CN').locale).toBe('zh');
    expect(parseDemoOptions('?scenario=feed&locale=jp').locale).toBe('ja');
    for (const params of ['?route=//evil.test', '?route=/rankings', '?tour=unknown', '?scenario=unknown', '?instance=../other', '?locale=xx']) expect(() => parseDemoOptions(params)).toThrow();
  });
});
