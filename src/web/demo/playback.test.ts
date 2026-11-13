import { describe, expect, it } from 'vitest';
import { createControlClient } from '@/shared/api/control-client.ts';
import { createDemoHost } from './runtime/host.ts';
import { createDemoPlayback, type PlaybackState } from './playback.ts';
import { demoTourScenarios, demoTourUrl } from './tours.ts';

function fixture() {
  const host = createDemoHost('connection');
  const client = createControlClient({ transport: host.transport });
  client.attach();
  const states: PlaybackState[] = [];
  const playback = createDemoPlayback(host, client, (state) => states.push(state));
  return { host, client, playback, states, dispose() { playback.dispose(); client.detach(); host.dispose(); } };
}

describe('demo playback controls', () => {
  it('connects through RPC before the first visible event and pauses/resumes coherently', async () => {
    const f = fixture();
    try {
      await f.playback.play();
      expect(f.host.wasSuccessful('live.connect')).toBe(true);
      expect(f.states.at(-1)).toEqual({ paused: false, busy: false, events: 1, status: 'playing' });
      await f.playback.play();
      f.playback.tick();
      expect(f.host.diagnostics().ticks).toBe(1);
      await f.playback.play();
      f.playback.tick();
      expect(f.states.at(-1)).toMatchObject({ events: 3, status: 'playing' });
    } finally { f.dispose(); }
  });
  it('Next event also connects and advances exactly once while remaining paused', async () => {
    const f = fixture();
    try {
      await Promise.all([f.playback.step(), f.playback.step()]);
      expect(f.host.diagnostics().ticks).toBe(1);
      expect(f.states.at(-1)).toEqual({ paused: true, busy: false, events: 1, status: 'paused' });
    } finally { f.dispose(); }
  });
  it('stops at the event bound and reports a disconnect instead of silently playing', async () => {
    const f = fixture();
    try {
      await f.playback.play();
      await f.client.call('live.disconnect', {});
      f.playback.tick();
      expect(f.states.at(-1)).toMatchObject({ paused: true, status: 'disconnected' });
      await f.playback.play();
      for (let index = 0; index < 310; index++) { f.playback.tick(); await Promise.resolve(); }
      expect(f.states.at(-1)).toMatchObject({ events: 300, paused: true, status: 'limit' });
    } finally { f.dispose(); }
  });
  it('disposal during the connect handshake prevents events and late UI callbacks', async () => {
    const f = fixture();
    const pending = f.playback.play();
    f.playback.dispose();
    const count = f.states.length;
    await pending;
    f.playback.tick();
    expect(f.host.diagnostics().ticks).toBe(0);
    expect(f.states).toHaveLength(count);
    f.dispose();
  });
  it('keeps RPC errors visible and releases the busy state', async () => {
    const f = fixture();
    f.client.detach();
    try {
      await f.playback.play();
      expect(f.states.at(-1)).toMatchObject({ busy: false, paused: true, status: 'error' });
    } finally { f.dispose(); }
  });
});

it('each selectable walkthrough prepares the matching fixtures and preserves locale/instance/base', () => {
  for (const id of Object.keys(demoTourScenarios) as Array<keyof typeof demoTourScenarios>) {
    const url = new URL(demoTourUrl('https://example.test/sub/demo/?locale=fr&instance=own&scenario=connection&route=/feed#/feed', id));
    expect(url.pathname).toBe('/sub/demo/');
    expect(url.searchParams.get('scenario')).toBe(demoTourScenarios[id]);
    expect(url.searchParams.get('tour')).toBe(id);
    expect(url.searchParams.get('locale')).toBe('fr');
    expect(url.searchParams.get('instance')).toBe('own');
    expect(url.searchParams.has('route')).toBe(false);
  }
});
