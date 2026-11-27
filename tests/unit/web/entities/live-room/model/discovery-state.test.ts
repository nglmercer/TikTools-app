import { describe, expect, it, vi } from 'vitest';

import type { ControlClient, RpcParams } from '@/shared/api/control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import type { LiveChannel } from '@/types.ts';
import { createDiscoveryState } from '../../../../../../src/web/entities/live-room/model/discovery-state.ts';
import { RPC } from '../../../../../shared/index.ts';

function stubControl(respond: (method: string, params?: RpcParams) => unknown): {
  control: ControlClient;
  calls: Array<{ method: string; params?: RpcParams }>;
} {
  const calls: Array<{ method: string; params?: RpcParams }> = [];
  const control: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: <T,>(method: string, params?: RpcParams): Promise<T> => {
      calls.push({ method, params });
      try {
        return Promise.resolve(respond(method, params) as T);
      } catch (failure) {
        return Promise.reject(failure as Error) as Promise<T>;
      }
    },
    onTopic: () => () => {},
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  return { control, calls };
}

function page(rooms: LiveChannel[], nextOffset?: number) {
  return { rooms, nextOffset, fetchedAt: Date.now(), source: 'tiktok-live-search' };
}

function channel(uniqueId: string, roomId: string, viewers = 1): LiveChannel {
  return { uniqueId, roomId, nickname: uniqueId.toUpperCase(), title: `t-${uniqueId}`, viewers };
}

const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));
const wait = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms));

describe('createDiscoveryState', () => {
  it('performs no request until the explorer is opened', async () => {
    const { control, calls } = stubControl(() => page([]));
    const discovery = createDiscoveryState(control);

    await flush();
    expect(calls).toHaveLength(0);

    discovery.setOpen(true);
    await flush();
    expect(calls).toHaveLength(1);
    expect(calls[0]?.method).toBe(RPC.liveChannels);
  });

  it('an unauthenticated refusal is not reported as no matches', async () => {
    const { control } = stubControl(() => {
      throw new ControlCallError('unauthenticated', 'TikTok requires a signed-in session');
    });
    const discovery = createDiscoveryState(control);

    discovery.setOpen(true);
    await flush();

    expect(discovery.status.value).toBe('auth_required');
    expect(discovery.rooms.value).toHaveLength(0);
  });

  it('rate limiting is distinct from a generic failure', async () => {
    const { control } = stubControl(() => {
      throw new ControlCallError('rate_limited', 'slow down');
    });
    const discovery = createDiscoveryState(control);

    discovery.setOpen(true);
    await flush();

    expect(discovery.status.value).toBe('rate_limited');
  });

  it('a generic failure keeps its message and yields no rooms', async () => {
    const { control } = stubControl(() => {
      throw new ControlCallError('timeout', 'network unreachable');
    });
    const discovery = createDiscoveryState(control);

    discovery.setOpen(true);
    await flush();

    expect(discovery.status.value).toBe('error');
    expect(discovery.error.value).toBe('network unreachable');
    expect(discovery.rooms.value).toHaveLength(0);
  });

  it('rooms are deduped by roomId, preserving order', async () => {
    const { control } = stubControl(() =>
      page([channel('a', '1'), channel('b', '2'), channel('a-again', '1')]),
    );
    const discovery = createDiscoveryState(control);

    discovery.setOpen(true);
    await flush();

    expect(discovery.rooms.value.map((room) => room.roomId)).toEqual(['1', '2']);
    expect(discovery.status.value).toBe('ready');
  });

  it('rapid keyword typing debounces into one request', async () => {
    const { control, calls } = stubControl(() => page([channel('a', '1')]));
    const discovery = createDiscoveryState(control);

    discovery.setOpen(true);
    await flush();
    const afterOpen = calls.length;

    discovery.setKeyword('m');
    discovery.setKeyword('mu');
    discovery.setKeyword('mus');
    await wait(450);

    expect(calls.length - afterOpen).toBe(1);
    expect(calls.at(-1)?.params).toMatchObject({ keyword: 'mus' });
  });

  it('a warm page is reused instead of refetching on reopen', async () => {
    const { control, calls } = stubControl(() => page([channel('a', '1')]));
    const discovery = createDiscoveryState(control);

    discovery.setOpen(true);
    await flush();
    expect(calls).toHaveLength(1);

    discovery.setOpen(false);
    discovery.setOpen(true);
    await flush();
    expect(calls).toHaveLength(1);
  });

  it('an explicit refresh always goes to the network', async () => {
    const { control, calls } = stubControl(() => page([channel('a', '1')]));
    const discovery = createDiscoveryState(control);

    discovery.setOpen(true);
    await flush();
    discovery.refresh();
    await flush();

    expect(calls).toHaveLength(2);
  });

  it('random pick excludes the current creator', async () => {
    const { control } = stubControl(() => page([channel('current', '1'), channel('other', '2')]));
    const discovery = createDiscoveryState(control);
    discovery.setOpen(true);
    await flush();

    vi.spyOn(Math, 'random').mockReturnValue(0);
    expect(discovery.pickRandom('current')?.uniqueId).toBe('other');
  });

  it('random pick indexes the whole eligible pool, not just the first room', async () => {
    const { control } = stubControl(() => page([channel('a', '1'), channel('b', '2'), channel('c', '3')]));
    const discovery = createDiscoveryState(control);
    discovery.setOpen(true);
    await flush();

    vi.spyOn(Math, 'random').mockReturnValue(0.99);
    expect(discovery.pickRandom('')?.uniqueId).toBe('c');
  });

  it('random pick over an empty list reports no matches and returns null', async () => {
    const { control } = stubControl(() => page([]));
    const discovery = createDiscoveryState(control);
    discovery.setOpen(true);
    await flush();

    expect(discovery.pickRandom('')).toBeNull();
    expect(discovery.notice.value).toBe('noMatches');
  });

  it('dispose cancels a pending debounced search', async () => {
    const { control, calls } = stubControl(() => page([]));
    const discovery = createDiscoveryState(control);
    discovery.setOpen(true);
    await flush();
    const before = calls.length;

    discovery.setKeyword('música');
    discovery.dispose();
    await wait(450);

    expect(calls.length).toBe(before);
  });

  it('a stale listing is flagged for revalidation', async () => {
    const { control } = stubControl(() => page([channel('a', '1')]));
    const discovery = createDiscoveryState(control);
    expect(discovery.isStale()).toBe(true);

    discovery.setOpen(true);
    await flush();
    expect(discovery.isStale()).toBe(false);
    discovery.markStale();
    expect(discovery.notice.value).toBe('stale');
  });
});
