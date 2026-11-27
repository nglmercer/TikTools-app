import { testMessage } from '@/shared/i18n/test-message.ts';
import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { useGlobalsStore } from '@/entities/runtime-global/model/globals-store.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import RuntimeGlobalsEditor from '../../../../../../src/web/features/edit-runtime-globals/ui/RuntimeGlobalsEditor.vue';
import { RPC } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);

async function mountEditor(handler: Parameters<typeof createFakeControlClient>[0], typed = false) {
  const { client, calls } = createFakeControlClient((method, params, options) => method === RPC.rpcDiscover
    ? { methods: (typed ? [RPC.globalsTypedList, RPC.globalsTypedReplace] : []).map((name) => ({ name })) }
    : handler(method, params, options));
  const harness = createStoreHarness(client);
  const capabilities = harness.use(() => useCapabilityStore());
  if (typed) await capabilities.refresh('typed-test');
  const store = harness.use(() => useGlobalsStore());
  const wrapper = mountWithPlugins(RuntimeGlobalsEditor, { global: { plugins: harness.plugins } });
  await flushPromises();
  return { wrapper, calls, store, capabilities };
}

type Wrapper = Awaited<ReturnType<typeof mountEditor>>['wrapper'];
const saveButton = (wrapper: Wrapper) => wrapper.get('button[aria-label="Save globals"]');
const discardButton = (wrapper: Wrapper) => wrapper.get('button[aria-label="Discard changes"]');

describe('RuntimeGlobalsEditor', () => {
  it('loads globals into editable rows', async () => {
    const { wrapper } = await mountEditor(() => ({ globals: { commandPort: '46665' } }));
    const inputs = wrapper.findAll('input');
    expect(inputs.map((input) => input.element.value)).toEqual(['commandPort', '46665']);
    expect(saveButton(wrapper).attributes('disabled')).toBeDefined();
    expect(discardButton(wrapper).attributes('disabled')).toBeDefined();
  });

  it('saves edited values through the store', async () => {
    const { wrapper, calls } = await mountEditor((method) =>
      method === RPC.globalsList ? { globals: { commandPort: '46665' } } : { ok: true },
    );
    await wrapper.findAll('input')[1]!.setValue('25575');
    await saveButton(wrapper).trigger('click');
    await flushPromises();
    expect(calls.at(-1)).toEqual({ method: RPC.globalsSet, params: { key: 'commandPort', value: '25575' } });
    expect(saveButton(wrapper).attributes('disabled')).toBeDefined();
  });

  it('explains invalid keys and blocks saving', async () => {
    const { wrapper } = await mountEditor(() => ({ globals: {} }));
    expect(wrapper.text()).toContain('No globals yet');
    await wrapper.get('button[aria-label="Add global"]').trigger('click');
    await wrapper.findAll('input')[0]!.setValue('9lives');
    expect(wrapper.get('[role="alert"]').text()).toContain('Keys start with a letter');
    expect(saveButton(wrapper).attributes('disabled')).toBeDefined();
  });

  it('removes the first global row and enables saving', async () => {
    const { wrapper } = await mountEditor(() => ({ globals: { a: '1', b: '2' } }));
    await wrapper.findAll('button[aria-label="Remove global"]')[0]!.trigger('click');
    expect(wrapper.findAll('input').map((input) => input.element.value)).toEqual(['b', '2']);
    expect(saveButton(wrapper).attributes('disabled')).toBeUndefined();
  });

  it('offers save, discard and add as icon buttons and keeps the explanation in a tooltip', async () => {
    const { wrapper } = await mountEditor(() => ({ globals: { commandPort: '46665' } }));
    expect(wrapper.findAll('header button').map((button) => button.text())).toEqual(['', '', '']);
    expect(wrapper.findAll('header button').map((button) => button.attributes('aria-label'))).toEqual([
      t('globals.add'), t('globals.discard'), t('globals.save'),
    ]);
    expect(wrapper.get('header [role="img"]').attributes('aria-label')).toContain('{{ globals.key }}');
  });

  it('discards unsaved edits and returns to the stored values', async () => {
    const { wrapper, calls } = await mountEditor(() => ({ globals: { commandPort: '46665' } }));
    await wrapper.findAll('input')[1]!.setValue('25575');
    expect(discardButton(wrapper).attributes('disabled')).toBeUndefined();
    await discardButton(wrapper).trigger('click');
    expect(wrapper.findAll('input').map((input) => input.element.value)).toEqual(['commandPort', '46665']);
    expect(saveButton(wrapper).attributes('disabled')).toBeDefined();
    expect(calls.some((call) => call.method === RPC.globalsSet)).toBe(false);
  });

  it('edits explicit types and masks secrets while saving only through the local typed API', async () => {
    const secret = 'local-editor-private-value';
    const revision = 'a'.repeat(64);
    const { wrapper, calls } = await mountEditor((method, params) => {
      if (method === RPC.globalsTypedList) return { revision, variables: { port: { type: 'text', value: '8080' }, token: { type: 'secret', value: secret } } };
      if (method === RPC.globalsTypedReplace) return { revision: 'b'.repeat(64), variables: params?.variables };
      throw new Error(`unexpected method ${method}`);
    }, true);
    const rows = wrapper.findAll('[role="row"]').slice(1);
    expect(rows[1]!.get('input[aria-label="Value"]').attributes('type')).toBe('password');
    await rows[0]!.get('select[aria-label="Type"]').setValue('number');
    await rows[0]!.get('input[aria-label="Value"]').setValue('46665');
    await saveButton(wrapper).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.globalsTypedReplace)).toEqual({ method: RPC.globalsTypedReplace, params: {
      expectedRevision: revision,
      variables: { port: { type: 'number', value: 46665 }, token: { type: 'secret', value: secret } },
    } });
    expect(calls.some((call) => call.method.startsWith('sync.') || call.method === RPC.globalsSet)).toBe(false);
    expect(saveButton(wrapper).attributes('disabled')).toBeDefined();
    expect(JSON.stringify(localStorage)).not.toContain(secret);
  });

  it('retains a dirty draft and its original revision after a conflicting save and a fresh host read', async () => {
    let revision = 'a'.repeat(64);
    const { wrapper, calls, store } = await mountEditor((method) => {
      if (method === RPC.globalsTypedList) return { revision, variables: { port: { type: 'number', value: revision.startsWith('a') ? 1 : 3 } } };
      if (method === RPC.globalsTypedReplace) {
        revision = 'b'.repeat(64);
        throw new ControlCallError('conflict', 'runtime variables changed; reload before saving');
      }
      throw new Error(`unexpected method ${method}`);
    }, true);
    await wrapper.get('input[aria-label="Value"]').setValue('2');
    await saveButton(wrapper).trigger('click');
    await flushPromises();
    await store.loadTyped();
    await flushPromises();
    expect(wrapper.get<HTMLInputElement>('input[aria-label="Value"]').element.value).toBe('2');
    await saveButton(wrapper).trigger('click');
    await flushPromises();
    expect(calls.filter((call) => call.method === RPC.globalsTypedReplace).map((call) => call.params?.expectedRevision))
      .toEqual(['a'.repeat(64), 'a'.repeat(64)]);
    await discardButton(wrapper).trigger('click');
    await flushPromises();
    expect(wrapper.get<HTMLInputElement>('input[aria-label="Value"]').element.value).toBe('3');
    expect(saveButton(wrapper).attributes('disabled')).toBeDefined();
  });

  it('blocks a wrong typed value before calling the host', async () => {
    const { wrapper, calls } = await mountEditor(() => ({ revision: 'a'.repeat(64), variables: { port: { type: 'text', value: 'not-number' } } }), true);
    await wrapper.get('select[aria-label="Type"]').setValue('number');
    expect(wrapper.get('[role="alert"]').text()).toContain('Enter a finite number');
    expect(saveButton(wrapper).attributes('disabled')).toBeDefined();
    expect(calls.some((call) => call.method === RPC.globalsTypedReplace)).toBe(false);
  });

  it('keeps a secret masked and blocks legacy saving when host capabilities change', async () => {
    const { wrapper, calls, capabilities } = await mountEditor((method) => {
      if (method === RPC.globalsTypedList) return { revision: 'a'.repeat(64), variables: { ordinary: { type: 'secret', value: 'private-draft' } } };
      if (method === RPC.globalsList) return { globals: { otherHost: 'public' } };
      throw new Error(`unexpected method ${method}`);
    }, true);
    await wrapper.get('input[aria-label="Value"]').setValue('unsaved-private-draft');
    capabilities.invalidate();
    await flushPromises();
    expect(wrapper.get('input[aria-label="Value"]').attributes('type')).toBe('password');
    expect(wrapper.get<HTMLInputElement>('input[aria-label="Value"]').element.value).toBe('unsaved-private-draft');
    expect(saveButton(wrapper).attributes('disabled')).toBeDefined();
    expect(wrapper.get('[role="alert"]').text()).toContain('This host does not support these variable types');
    expect(calls.some((call) => call.method === RPC.globalsSet || call.method === RPC.globalsTypedReplace)).toBe(false);
    await discardButton(wrapper).trigger('click');
    await flushPromises();
    expect(wrapper.findAll('input').map((input) => input.element.value)).toEqual(['otherHost', 'public']);
  });
});
