import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { createApp, nextTick } from 'vue';
import type { UiEvent } from '../../../../shared/messages.ts';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { useLiveFeedStore } from '@/entities/live-event/model/live-feed-store.ts';
import type { ControlClient, TopicHandler } from '@/shared/api/control-client.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { createPinia, setActivePinia } from 'pinia';
import FeedPage from './FeedPage.vue';

enableAutoUnmount(afterEach);

beforeEach(() => {
  localStorage.clear();
});

function harness() {
  const topics = new Map<string, Set<TopicHandler>>();
  const client: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: (async () => ({})) as ControlClient['call'],
    onTopic: (topic, handler) => {
      const set = topics.get(topic) ?? new Set();
      set.add(handler as TopicHandler);
      topics.set(topic, set);
      return () => set.delete(handler as TopicHandler);
    },
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  const emit = (topic: string, data: unknown) => topics.get(topic)?.forEach((handler) => handler(data));
  return { client, emit };
}

async function mountFeed() {
  const { client, emit } = harness();
  const pinia = createPinia();
  setActivePinia(pinia);
  const app = createApp({});
  app.use(pinia).provide(controlClientKey, client);
  const store = app.runWithContext(() => useLiveFeedStore());
  app.runWithContext(() => useConnectionStore()).activeCreator = 'luna';
  const wrapper = mountWithPlugins(FeedPage, {
    attachTo: document.body,
    global: {
      plugins: [pinia],
      provide: { [controlClientKey as symbol]: client },
    },
  });
  const push = async (event: UiEvent) => {
    emit('live.ui-event', { event });
    await new Promise((resolve) => setTimeout(resolve, 120));
    await flushPromises();
  };
  return { wrapper, store, push, emit };
}

describe('FeedPage', () => {
  it('shows an empty stream until events arrive', async () => {
    const { wrapper } = await mountFeed();
    expect(wrapper.text()).toContain('No live events yet');
    expect(wrapper.text()).toContain('0 events');
  });

  it('renders incoming events and filters them by kind and text', async () => {
    const { wrapper, push } = await mountFeed();
    await push({ kind: 'chat', author: 'luna', text: 'hola a todos', msgId: '1' });
    await push({ kind: 'gift', author: 'mango', text: 'Rose', i18nKey: 'giftSent', giftDetails: { name: 'Rose', count: 2, diamonds: 1 }, msgId: '2' });
    expect(wrapper.findAll('.event-row')).toHaveLength(2);
    expect(wrapper.text()).toContain('sent 2× Rose');
    expect(wrapper.text()).toContain('2 events');
    await wrapper.findAll('.base-segmented__option').find((option) => option.text() === 'Gifts')!.trigger('click');
    expect(wrapper.findAll('.event-row')).toHaveLength(1);
    await wrapper.findAll('.base-segmented__option')[0]!.trigger('click');
    await wrapper.get('input[type="search"]').setValue('hola');
    expect(wrapper.findAll('.event-row').map((row) => row.text())).toEqual([expect.stringContaining('hola a todos')]);
  });

  it('switches spectator rankings to core like totals and opens the viewer', async () => {
    const { wrapper, emit } = await mountFeed();
    emit('live.connected', { uniqueId: 'luna', roomId: 'room-42' });
    emit('live.likes-ranking', {
      creatorId: 'luna',
      roomId: 'room-42',
      leaders: [
        { rank: 1, score: 164, uniqueId: 'alice', nickname: 'Alice', userId: '101' },
        { rank: 2, score: 100, uniqueId: 'bob', nickname: 'Bob', userId: '102' },
      ],
    });
    await nextTick();
    expect(wrapper.find('.top-likes__row').exists()).toBe(false);
    const likesFilter = wrapper.findAll('.feed-inspector__viewer-filter button')
      .find((button) => button.text() === 'Likes');
    expect(likesFilter).toBeDefined();
    await likesFilter!.trigger('click');
    expect(wrapper.findAll('.top-likes__row')).toHaveLength(2);
    expect(wrapper.find('.top-likes__row').text()).toContain('164');
    await wrapper.find('.top-likes__row').trigger('click');
    expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe('Profile');
    expect(wrapper.text()).toContain('Alice');
  });

  it('opens event details and the author profile in the inspector', async () => {
    const { wrapper, push } = await mountFeed();
    await push({ kind: 'chat', author: 'luna', nickname: 'Luna', text: 'first', msgId: 'a' });
    await push({ kind: 'chat', author: 'luna', nickname: 'Luna', text: 'second', msgId: 'b' });
    await wrapper.findAll('.event-row__content')[0]!.trigger('click');
    expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe('Details');
    expect(wrapper.text()).toContain('Message ID');
    expect(wrapper.findAll('.event-row')[0]!.classes()).toContain('event-row--selected');
    await wrapper.findAll('.event-row__author')[1]!.trigger('click');
    expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe('Profile');
    expect(wrapper.findAll('.feed-inspector__message').map((item) => item.text())).toEqual([
      expect.stringContaining('second'),
      expect.stringContaining('first'),
    ]);
  });

  it('pauses auto-scroll and offers a jump back with the unread count', async () => {
    const { wrapper, push } = await mountFeed();
    await wrapper.get('button[aria-label="Pause auto-scroll"]').trigger('click');
    await push({ kind: 'chat', author: 'luna', text: 'while paused', msgId: 'p1' });
    expect(wrapper.text()).toContain('Jump to latest (1)');
    await wrapper.findAll('button').find((button) => button.text().startsWith('Jump to latest'))!.trigger('click');
    await nextTick();
    expect(wrapper.text()).not.toContain('Jump to latest');
  });

  it('hides and restores the inspector', async () => {
    const { wrapper } = await mountFeed();
    expect(wrapper.find('.feed-inspector').exists()).toBe(true);
    await wrapper.get('button[aria-label="Hide inspector"]').trigger('click');
    expect(wrapper.find('.feed-inspector').exists()).toBe(false);
    await wrapper.get('button[aria-label="Show inspector"]').trigger('click');
    expect(wrapper.find('.feed-inspector').exists()).toBe(true);
  });

  it('clears the stream', async () => {
    const { wrapper, push } = await mountFeed();
    await push({ kind: 'chat', author: 'luna', text: 'bye', msgId: 'c1' });
    await wrapper.get('button[aria-label="Clear feed"]').trigger('click');
    await nextTick();
    expect(wrapper.findAll('.event-row')).toHaveLength(0);
  });
});
