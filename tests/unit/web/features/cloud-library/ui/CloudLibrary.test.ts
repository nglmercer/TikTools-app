const t = (key: string): string => testMessage('en', key);
import { testMessage } from '@/shared/i18n/test-message.ts';
import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import CloudLibrary from '../../../../../../src/web/features/cloud-library/ui/CloudLibrary.vue';
import { ACCOUNT_STATE, ACTION_TYPE, EVENT_KIND, RPC } from '../../../../../shared/index.ts';

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
    account.status = { origin: 'https://cloud.example', state: ACCOUNT_STATE.signedIn, accountId: 'alice', persistence: ACCOUNT_STATE.memoryOnly };
    const wrapper = mountWithPlugins(CloudLibrary, { global: { plugins: stores.plugins } });
    await flushPromises();
    await wrapper.findAll('select')[0]!.setValue('mine');
    expect(wrapper.text()).toContain('Loading Cloud documents');
    account.working = true;
    account.status = { origin: 'https://cloud.example', state: ACCOUNT_STATE.signedOut, persistence: ACCOUNT_STATE.memoryOnly };
    account.status = { origin: 'https://cloud.example', state: ACCOUNT_STATE.signedIn, accountId: 'alice', persistence: ACCOUNT_STATE.memoryOnly };
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
    account.status = { origin: 'https://cloud.example', state: ACCOUNT_STATE.signedIn, accountId: 'alice', persistence: ACCOUNT_STATE.memoryOnly };
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
    client.onGap = (handler) => {
      gap = () => handler(1);
      return () => { gap = () => {}; };
    };
    const stores = createStoreHarness(client);
    const account = stores.use(() => useAccountStore());
    account.status = { origin: 'https://cloud.example', state: ACCOUNT_STATE.signedIn, accountId: 'alice', persistence: ACCOUNT_STATE.memoryOnly };
    const wrapper = mountWithPlugins(CloudLibrary, { global: { plugins: stores.plugins } });
    await flushPromises();
    await wrapper.findAll('select')[0]!.setValue('mine');
    await flushPromises();
    await button(wrapper, 'Private title').trigger('click');
    cloudInvalidated();
    await nextTick();
    expect(wrapper.text()).not.toContain('Private title');
    resolveDetail({ meta: summary(firstId, 'Private title'), document: { title: 'Old detail' } });
    await flushPromises();
    expect(wrapper.text()).not.toContain('Old detail');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(wrapper.text()).toContain('Private title');
    gap();
    await nextTick();
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

  describe('import', () => {
    const template = { templateVersion: 1, id: 'portable-template', title: 'Safe example', description: '', icon: 'gift',
      actions: [{ name: 'act', typeId: ACTION_TYPE.coreLog, config: {} }], event: { name: 'evt', trigger: EVENT_KIND.tiktokChat } };
    async function ownerLibrary(hold?: { snapshot?: Promise<unknown> }) {
      const native = nativeDefinitionHost((method, params, options) => {
        if (method === RPC.automationSnapshot) {
          options?.onMetadata?.({ resourceVersions: [] });
          return { actions: [], events: [], plugins: [], actionTypes: [], translations: {} };
        }
        if (method === RPC.appStateGet) return { state: {} };
        if (method === 'cloudLibrary.list') return { items: [summary()], nextCursor: null };
        if (method === 'cloudLibrary.read') return { meta: summary(), document: template };
        if (method === RPC.automationCreate) {
          options?.onMetadata?.({
            resourceVersions: [{ kind: 'automation', scope: params?.kind as string, id: (params!.record as { id: string }).id, revision: '1' }],
          });
          return params?.record;
        }
        throw new Error(`unexpected ${method}`);
      });
      const { client, calls } = createFakeControlClient(async (method, params, options) => {
        // The library read that follows the already-open document is held.
        if (hold?.snapshot && method === RPC.automationLibrarySnapshot) await hold.snapshot;
        return native.handle(method, params, options);
      });
      const stores = createStoreHarness(client);
      const account = stores.use(() => useAccountStore());
      account.status = { origin: 'https://cloud.example', state: ACCOUNT_STATE.signedIn, accountId: 'alice', persistence: ACCOUNT_STATE.memoryOnly };
      const wrapper = mountWithPlugins(CloudLibrary, { global: { plugins: stores.plugins } });
      await flushPromises();
      await button(wrapper, 'First document').trigger('click');
      await flushPromises();
      return { wrapper, account, calls, journal: libraryJournal(client) };
    }

    it('opens the owner review without leaving the library, and shows the result only after the owner confirms', async () => {
      const f = await ownerLibrary();
      await button(f.wrapper, t('cloudLibrary.import')).trigger('click');
      await flushPromises();
      expect(f.journal.draft.value?.provenance).toMatchObject({
        origin: 'https://cloud.example',
        accountId: 'alice',
        documentId: firstId,
        revision: '9007199254740993',
        sha256: 'a'.repeat(64),
      });
      expect(f.wrapper.text()).not.toContain(t('cloudLibrary.staged'));
      expect(button(f.wrapper, t('cloudLibrary.import')).attributes('disabled')).toBeDefined();
      expect(f.calls.filter((call) => call.method === 'cloudLibrary.read')).toHaveLength(1);
      await f.journal.confirm();
      await flushPromises();
      expect(f.wrapper.text()).toContain(t('cloudLibrary.staged'));
      expect(button(f.wrapper, t('cloudLibrary.import')).attributes('disabled')).toBeUndefined();
      expect(f.calls.filter((call) => call.method === RPC.automationLibrarySubmit)).toHaveLength(1);
      f.wrapper.unmount();
    });

    it('frees the import when the review is discarded and reports no error', async () => {
      const f = await ownerLibrary();
      await button(f.wrapper, t('cloudLibrary.import')).trigger('click');
      await flushPromises();
      f.journal.dismiss();
      await flushPromises();
      expect(button(f.wrapper, t('cloudLibrary.import')).attributes('disabled')).toBeUndefined();
      expect(f.wrapper.find('[role="alert"]').exists()).toBe(false);
      expect(f.calls.some((call) => call.method === RPC.automationLibrarySubmit)).toBe(false);
      f.wrapper.unmount();
    });

    it('does not prepare a draft when the account changes while the library is being read', async () => {
      let release!: () => void;
      const held = new Promise<void>((resolve) => { release = resolve; });
      const f = await ownerLibrary({ snapshot: held });
      await button(f.wrapper, t('cloudLibrary.import')).trigger('click');
      await flushPromises();
      f.account.working = true;
      f.account.status = { origin: 'https://cloud.example', state: ACCOUNT_STATE.signedIn, accountId: 'bob', persistence: ACCOUNT_STATE.memoryOnly };
      f.account.working = false;
      release();
      await flushPromises();
      expect(f.journal.draft.value).toBeNull();
      expect(f.calls.some((call) => call.method === RPC.automationLibrarySubmit)).toBe(false);
      f.wrapper.unmount();
    });
  });
});
