import { testMessage } from '@/shared/i18n/test-message.ts';
import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import OverlaysPage from '../../../../../../src/web/pages/overlays/ui/OverlaysPage.vue';
import { RPC, STORAGE_KEY } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);

beforeEach(() => {
  localStorage.clear();
});

async function mountPage(announce: string[] = []) {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === RPC.rpcDiscover) return { methods: announce.map((name) => ({ name })) };
    if (method === 'widgets.status') return { state: 'ready', port: 19999, error: null };
    if (method === RPC.appStateGet) return { state: { 'widgets.design.follow': '{"radius":24}' } };
    if (method === 'widgets.copyObsUrl') return { ok: true };
    return { ok: true };
  });
  const stores = createStoreHarness(client);
  if (announce.length > 0) await stores.use(() => useCapabilityStore()).refresh('test-host');
  const wrapper = mountWithPlugins(OverlaysPage, { attachTo: document.body, global: { plugins: stores.plugins } });
  await flushPromises();
  return { wrapper, calls };
}

type Wrapper = Awaited<ReturnType<typeof mountPage>>['wrapper'];

function buttonByText(wrapper: Wrapper, text: string) {
  return wrapper.findAll('button').find((button) => button.text() === text)!;
}

async function selectKind(wrapper: Wrapper, name: string) {
  await wrapper.findAll('.overlays-page__kinds [role="option"]').find((item) => item.text().includes(name))!.trigger('pointerdown');
  await nextTick();
}

describe('OverlaysPage', () => {
  it('shows the gateway status and the redacted source url', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.get('.overlays-page__gateway').text()).toContain('Ready on port 19999');
    expect((wrapper.get('#overlay-source-url').element as HTMLInputElement).value).toBe(
      'http://127.0.0.1:19999/widgets/follow/#token=••••••••',
    );
  });

  it('does not expose the obsolete portable package dialog', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.text()).not.toContain(t('overlays.portablePackages'));
    expect(wrapper.find('.overlays-page__portable').exists()).toBe(false);
  });

  it('offers the Cloud area only when the host announces overlays or packages', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.find('button[aria-label="Cloud"]').exists()).toBe(false);
    const packages = await mountPage([RPC.bundlesPackagesPreflight]);
    expect(packages.wrapper.find('button[aria-label="Cloud"]').exists()).toBe(true);
  });

  it('opens the overlay library in a modal and returns to the retained design', async () => {
    const { wrapper } = await mountPage([RPC.bundlesOverlaysCreate, RPC.bundlesPackagesPreflight]);
    const source = (wrapper.get('#overlay-source-url').element as HTMLInputElement).value;
    await wrapper.get('button[aria-label="Cloud"]').trigger('click');
    expect(wrapper.get('dialog.overlay-library').attributes('open')).toBeDefined();
    expect(wrapper.findAll('.overlay-library__card')).toHaveLength(7);
    expect(wrapper.get('.overlays-page__workspace').isVisible()).toBe(true);
    await wrapper.get('.overlay-library button[aria-label="Close"]').trigger('click');
    expect(wrapper.find('.overlay-library__card').exists()).toBe(false);
    expect((wrapper.get('#overlay-source-url').element as HTMLInputElement).value).toBe(source);
  });

  it('shows only the Cloud feature the host announced', async () => {
    const { wrapper } = await mountPage([RPC.bundlesPackagesPreflight]);
    await wrapper.get('button[aria-label="Cloud"]').trigger('click');
    expect(wrapper.find('.cloud-overlays').exists()).toBe(false);
    await buttonByText(wrapper, t('overlayLibrary.images')).trigger('click');
    expect(wrapper.find('.cloud-bundles').exists()).toBe(true);
  });

  it('filters library cards and opens the selected widget without losing the workspace', async () => {
    const { wrapper } = await mountPage([RPC.bundlesOverlaysCreate]);
    await wrapper.get('button[aria-label="Cloud"]').trigger('click');
    expect(buttonByText(wrapper, t('overlayLibrary.open')).attributes('disabled')).toBeDefined();
    await wrapper.get('.overlay-library input[type="search"]').setValue('gift');
    expect(wrapper.findAll('.overlay-library__card')).toHaveLength(1);
    await wrapper.get('.overlay-library__card').trigger('click');
    expect(buttonByText(wrapper, t('overlayLibrary.open')).attributes('disabled')).toBeUndefined();
    await buttonByText(wrapper, t('overlayLibrary.open')).trigger('click');
    expect(wrapper.find('.overlay-library__card').exists()).toBe(false);
    expect((wrapper.get('#overlay-source-url').element as HTMLInputElement).value).toContain('/widgets/gift/');
  });

  it('clears selection when search hides the selected overlay', async () => {
    const { wrapper } = await mountPage([RPC.bundlesOverlaysCreate]);
    await wrapper.get('button[aria-label="Cloud"]').trigger('click');
    await wrapper.findAll('.overlay-library__card')[0]!.trigger('click');
    await wrapper.get('.overlay-library input[type="search"]').setValue('no matching widget');
    expect(wrapper.get('.overlay-library').text()).toContain(t('overlayLibrary.empty'));
    expect(buttonByText(wrapper, t('overlayLibrary.open')).attributes('disabled')).toBeDefined();
  });

  it('copies the source url through the host', async () => {
    const { wrapper, calls } = await mountPage();
    await buttonByText(wrapper, t('overlays.source.copy')).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'widgets.copyObsUrl')?.params).toEqual({ widget: 'follow' });
    expect(buttonByText(wrapper, 'Copied').exists()).toBe(true);
  });

  it('edits and saves the selected design', async () => {
    const { wrapper, calls } = await mountPage();
    expect(buttonByText(wrapper, t('overlays.editor.save')).attributes('disabled')).toBeDefined();
    await buttonByText(wrapper, t('overlays.editor.sections.card')).trigger('click');
    const shadow = wrapper.findAll('.design-inspector input[type="checkbox"]')[0]!;
    await shadow.setValue(false);
    expect(buttonByText(wrapper, t('overlays.editor.save')).attributes('disabled')).toBeUndefined();
    await buttonByText(wrapper, t('overlays.editor.save')).trigger('click');
    await flushPromises();
    const saved = calls.find((call) => call.method === RPC.appStateSet);
    expect(saved?.params).toMatchObject({ key: 'widgets.design.follow' });
    expect(JSON.parse(String(saved?.params?.value))).toMatchObject({ radius: 24 });
    expect(wrapper.text()).toContain(t('overlays.editor.saved'));
  });

  it('asks before switching away from unsaved changes', async () => {
    const { wrapper } = await mountPage();
    await buttonByText(wrapper, t('overlays.editor.sections.card')).trigger('click');
    await wrapper.findAll('.design-inspector input[type="checkbox"]')[0]!.setValue(false);
    await selectKind(wrapper, t('overlays.kinds.gift.name'));
    expect(document.body.textContent).toContain(t('overlays.editor.discardTitle'));
    Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) => button.textContent?.trim() === 'Discard')!.click();
    await nextTick();
    expect((wrapper.get('#overlay-source-url').element as HTMLInputElement).value).toContain('/widgets/gift/');
  });

  it('switches directly when nothing changed and remembers the choice', async () => {
    const { wrapper } = await mountPage();
    await selectKind(wrapper, t('overlays.kinds.chat.name'));
    expect((wrapper.get('#overlay-source-url').element as HTMLInputElement).value).toContain('/widgets/chat/');
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY.viewState) ?? '{}').widgets).toEqual({ selectedKind: 'chat' });
  });
});
