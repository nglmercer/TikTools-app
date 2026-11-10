import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CloudLibrary from './CloudLibrary.vue';

const firstId = '00000000-0000-4000-8000-000000000001';
const nextId = '00000000-0000-4000-8000-000000000002';
const summary = (id = firstId, title = 'First document') => ({
  id, kind: 'template', slug: 'first', revision: '9007199254740993', title,
  sha256: 'a'.repeat(64), status: 'published',
});
const button = (wrapper: ReturnType<typeof mountWithPlugins>, text: string) =>
  wrapper.findAll('button').find((item) => item.text() === text)!;

describe('CloudLibrary', () => {
  it('browses pages and reads the exact revision without importing or activating', async () => {
    const { client, calls } = createFakeControlClient((method, params) => {
      if (method === 'cloudLibrary.list') return params?.after
        ? { items: [summary(nextId, 'Second document')], nextCursor: null }
        : { items: [summary()], nextCursor: nextId };
      if (method === 'cloudLibrary.read') return { meta: summary(), document: { id: 'portable-template', title: 'Safe example' } };
      throw new Error(`unexpected ${method}`);
    });
    const stores = createStoreHarness(client);
    const wrapper = mountWithPlugins(CloudLibrary, { global: { plugins: stores.plugins } });
    await flushPromises();
    expect(wrapper.text()).toContain('First document');
    expect(wrapper.text()).toContain('9007199254740993');
    await button(wrapper, 'First document').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('Safe example');
    expect(calls.at(-1)).toEqual({ method: 'cloudLibrary.read', params: { scope: 'public', id: firstId, revision: '9007199254740993' } });
    // An unsubmitted filter must not be combined with the previous cursor.
    await wrapper.get('input[type="search"]').setValue('new filter');
    await button(wrapper, 'Next').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('Second document');
    expect(wrapper.text()).not.toContain('Safe example');
    expect(calls.at(-1)).toEqual({ method: 'cloudLibrary.list', params: { scope: 'public', kind: null, query: null, after: nextId } });
    await button(wrapper, 'Previous').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('First document');
    expect(calls.every(({ method }) => ['cloudLibrary.list', 'cloudLibrary.read'].includes(method))).toBe(true);
    wrapper.unmount();
  });

  it('ignores a delayed private response after logout and same-account login', async () => {
    let resolvePrivate!: (value: unknown) => void;
    const pending = new Promise((resolve) => { resolvePrivate = resolve; });
    const { client } = createFakeControlClient((method, params) => {
      if (method === 'cloudLibrary.list' && params?.scope === 'mine') return pending;
      if (method === 'cloudLibrary.list') return { items: [], nextCursor: null };
      throw new Error(`unexpected ${method}`);
    });
    const stores = createStoreHarness(client);
    const account = stores.use(() => useAccountStore());
    account.status = { origin: 'https://cloud.example', state: 'signed-in', accountId: 'alice', persistence: 'memory-only' };
    const wrapper = mountWithPlugins(CloudLibrary, { global: { plugins: stores.plugins } });
    await flushPromises();
    await wrapper.findAll('select')[0]!.setValue('mine');
    expect(wrapper.text()).toContain('Loading Cloud documents');
    account.working = true;
    account.status = { origin: 'https://cloud.example', state: 'signed-out', persistence: 'memory-only' };
    account.status = { origin: 'https://cloud.example', state: 'signed-in', accountId: 'alice', persistence: 'memory-only' };
    account.working = false;
    resolvePrivate({ items: [summary(firstId, 'Private old session')], nextCursor: null });
    await flushPromises();
    expect(wrapper.text()).not.toContain('Private old session');
    expect(wrapper.text()).not.toContain('Loading Cloud documents');
    wrapper.unmount();
  });

  it('clears private details immediately and discards an older detail response', async () => {
    let resolveDetail!: (value: unknown) => void;
    const pending = new Promise((resolve) => { resolveDetail = resolve; });
    const { client } = createFakeControlClient((method, params) => {
      if (method === 'cloudLibrary.list') return params?.scope === 'mine'
        ? { items: [summary(firstId, 'Alice-private-001')], nextCursor: null }
        : { items: [], nextCursor: null };
      if (method === 'cloudLibrary.read') return pending;
      throw new Error(`unexpected ${method}`);
    });
    const stores = createStoreHarness(client);
    const account = stores.use(() => useAccountStore());
    account.status = { origin: 'https://cloud.example', state: 'signed-in', accountId: 'alice', persistence: 'memory-only' };
    const wrapper = mountWithPlugins(CloudLibrary, { global: { plugins: stores.plugins } });
    await flushPromises();
    await wrapper.findAll('select')[0]!.setValue('mine');
    await flushPromises();
    await button(wrapper, 'Alice-private-001').trigger('click');
    account.working = true;
    await nextTick();
    expect(wrapper.text()).not.toContain('Alice-private-001');
    resolveDetail({ meta: summary(firstId, 'Alice-private-001'), document: { title: 'Private detail' } });
    await flushPromises();
    expect(wrapper.text()).not.toContain('Private detail');
    wrapper.unmount();
  });

  it('clears private content on a same-host context event or event gap', async () => {
    let resolveDetail!: (value: unknown) => void;
    let cloudInvalidated = () => {};
    let gap = () => {};
    const pending = new Promise((resolve) => { resolveDetail = resolve; });
    const { client } = createFakeControlClient((method, params) => {
      if (method === 'cloudLibrary.list') return params?.scope === 'mine'
        ? { items: [summary(firstId, 'Private title')], nextCursor: null }
        : { items: [], nextCursor: null };
      if (method === 'cloudLibrary.read') return pending;
      throw new Error(`unexpected ${method}`);
    });
    client.onTopic = (topic, handler) => {
      if (topic === 'cloud.context.invalidated') cloudInvalidated = () => handler({} as never);
      return () => { cloudInvalidated = () => {}; };
    };
    client.onGap = (handler) => { gap = () => handler(1); return () => { gap = () => {}; }; };
    const stores = createStoreHarness(client);
    const account = stores.use(() => useAccountStore());
    account.status = { origin: 'https://cloud.example', state: 'signed-in', accountId: 'alice', persistence: 'memory-only' };
    const wrapper = mountWithPlugins(CloudLibrary, { global: { plugins: stores.plugins } });
    await flushPromises();
    await wrapper.findAll('select')[0]!.setValue('mine'); await flushPromises();
    await button(wrapper, 'Private title').trigger('click');
    cloudInvalidated(); await nextTick();
    expect(wrapper.text()).not.toContain('Private title');
    resolveDetail({ meta: summary(firstId, 'Private title'), document: { title: 'Old detail' } });
    await flushPromises();
    expect(wrapper.text()).not.toContain('Old detail');
    await wrapper.get('form').trigger('submit'); await flushPromises();
    expect(wrapper.text()).toContain('Private title');
    gap(); await nextTick();
    expect(wrapper.text()).not.toContain('Private title');
    wrapper.unmount();
  });

  it('rejects malformed pages and mismatched document revisions', async () => {
    let invalidPage = true;
    const { client } = createFakeControlClient((method) => {
      if (method === 'cloudLibrary.list') return invalidPage
        ? { items: [summary(firstId, '<invalid>'), summary(firstId, 'duplicate')], nextCursor: null }
        : { items: [summary()], nextCursor: null };
      if (method === 'cloudLibrary.read') return { meta: { ...summary(), revision: '9007199254740994' }, document: {} };
      throw new Error(`unexpected ${method}`);
    });
    const stores = createStoreHarness(client);
    const wrapper = mountWithPlugins(CloudLibrary, { global: { plugins: stores.plugins } });
    await flushPromises();
    expect(wrapper.get('[role="alert"]').text()).toContain('could not be loaded');
    expect(wrapper.text()).not.toContain('<invalid>');
    invalidPage = false;
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    await button(wrapper, 'First document').trigger('click');
    await flushPromises();
    expect(wrapper.get('[role="alert"]').text()).toContain('could not be loaded');
    expect(wrapper.find('pre').exists()).toBe(false);
    wrapper.unmount();
  });
});
