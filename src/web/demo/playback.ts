import type { ControlClient } from '@/shared/api/control-client.ts';
import type { createDemoHost } from './runtime/host.ts';

export type PlaybackStatus = 'ready' | 'connecting' | 'playing' | 'paused' | 'disconnected' | 'limit' | 'error';
export interface PlaybackState { paused: boolean; busy: boolean; events: number; status: PlaybackStatus }

/** Toolbar effects use the same simulated RPC contract as the real connection form. */
export function createDemoPlayback(host: ReturnType<typeof createDemoHost>, client: ControlClient, changed: (state: PlaybackState) => void) {
  let disposed = false;
  let busy = false;
  let currentStatus: PlaybackStatus = 'ready';
  function publish(status: PlaybackStatus) {
    currentStatus = status;
    if (!disposed) changed({ paused: host.paused, busy, events: host.diagnostics().ticks, status });
  }
  function pause() { host.setPaused(true); publish(host.diagnostics().ticks >= 300 ? 'limit' : 'paused'); }
  async function advance(autoplay: boolean) {
    if (disposed || busy) return;
    busy = true;
    try {
      if (!host.state.liveStatus.connected) {
        publish('connecting');
        await client.call('live.connect', { uniqueId: 'demo_creator' });
      }
      if (disposed) return;
      host.setPaused(!autoplay);
      const advanced = host.step();
      if (!advanced || host.diagnostics().ticks >= 300) host.setPaused(true);
      publish(host.diagnostics().ticks >= 300 ? 'limit' : advanced ? autoplay ? 'playing' : 'paused' : 'disconnected');
    } catch {
      host.setPaused(true);
      publish('error');
    } finally {
      busy = false;
      publish(currentStatus);
    }
  }
  return {
    play: () => host.paused ? advance(true) : pause(),
    step: () => { pause(); return advance(false); },
    pause,
    tick() {
      if (disposed || busy || host.paused) return;
      const advanced = host.tick();
      if (!advanced || host.diagnostics().ticks >= 300) host.setPaused(true);
      publish(host.diagnostics().ticks >= 300 ? 'limit' : advanced ? 'playing' : 'disconnected');
    },
    dispose() { disposed = true; host.setPaused(true); },
  };
}
