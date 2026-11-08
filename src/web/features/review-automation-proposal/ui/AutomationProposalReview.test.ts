import { enableAutoUnmount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { proposalFixture } from '../testing/fixture.ts';
import AutomationProposalReview from './AutomationProposalReview.vue';

enableAutoUnmount(afterEach);
beforeEach(() => activateLocale('en'));
afterEach(() => activateLocale('en'));
const button = (wrapper: VueWrapper, label: string) => wrapper.findAll('button').find((b) => b.text() === label)!;
async function mountReview(host = proposalFixture(), selected = true) {
  const wrapper = mountWithPlugins(AutomationProposalReview, {
    attachTo: document.body,
    props: { target: selected ? { kind: 'event', id: 'same' } : null, hostVersions: host.projection() },
    global: { provide: { [controlClientKey as symbol]: host.client } },
  });
  await flushPromises();
  return { host, wrapper };
}

describe('saved owner authority review', () => {
  it('warns about rounded source numbers and blocks owner adoption while retaining saved inspection', async () => {
    const host = proposalFixture();
    host.source.record!.filters = JSON.parse('[{"field":"event.data.count","value":9007199254740993}]') as unknown[];
    const { wrapper } = await mountReview(host);
    await button(wrapper, 'Review saved authority').trigger('click'); await flushPromises();
    expect(wrapper.get('[data-test="adoption-source-precision"]').text()).toContain('cannot display every saved integer exactly');
    expect(wrapper.text()).toContain('Saved proposal');
    expect(wrapper.get('[data-test="adoption-acknowledge"]').attributes('disabled')).toBeDefined();
    expect(wrapper.get('[data-test="adoption-submit"]').attributes('disabled')).toBeDefined();
    await activateLocale('es'); await nextTick();
    expect(wrapper.get('[data-test="adoption-source-precision"]').text()).toContain('valor redondeado');
  });

  it('shows the shared authority-transition blocker when a delegated approval is unresolved', async () => {
    const host = proposalFixture();
    host.approvalState.pending = { commandId: `owner-approval-${'1'.repeat(32)}`, operation: 'activate', kind: 'event', id: 'same', expectedRevision: '4', hostInstanceId: 'instance-a', createdAt: 1 };
    const { wrapper } = await mountReview(host);
    await button(wrapper, 'Review saved authority').trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('Resolve it in Approval history');
    expect(wrapper.get('[data-test="adoption-acknowledge"]').attributes('disabled')).toBeDefined();
    expect(wrapper.get('[data-test="adoption-submit"]').attributes('disabled')).toBeDefined();
    expect(host.calls.some((c) => c.method === 'automation.adoption.submit')).toBe(false);
  });

  it('loads definitions only on explicit review and escapes all source while including disabled/random/missing references', async () => {
    const { host, wrapper } = await mountReview();
    expect(host.calls.some((c) => c.method === 'automation.review')).toBe(false);
    await button(wrapper, 'Review saved authority').trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('Saved proposal');
    expect(wrapper.text()).toContain('Disabled guard');
    expect(wrapper.text()).toContain('No saved record exists at this ID.');
    expect(wrapper.text()).toContain('Referenced actions (3)');
    expect(wrapper.text()).toContain('<script>privateSource()</script>');
    expect(wrapper.find('script').exists()).toBe(false);
    expect(wrapper.get('[data-test="adoption-submit"]').attributes('disabled')).toBeDefined();
    await wrapper.get('[data-test="adoption-acknowledge"]').setValue(true);
    expect(wrapper.get('[data-test="adoption-submit"]').attributes('disabled')).toBeUndefined();
    await wrapper.get('[data-test="adoption-submit"]').trigger('click'); await flushPromises();
    expect(host.calls.filter((c) => c.method === 'automation.adoption.submit')).toHaveLength(1);
    expect(wrapper.text()).toContain('native-adoption-1');
    expect(host.calls.some((c) => c.method === 'automation.enable' || c.method === 'automation.update')).toBe(false);
  });

  it('removes acknowledgement after any referenced host revision changes', async () => {
    const { host, wrapper } = await mountReview();
    await button(wrapper, 'Review saved authority').trigger('click'); await flushPromises();
    await wrapper.get('[data-test="adoption-acknowledge"]').setValue(true);
    const changed = host.projection(); changed.set('action:disabled', '4');
    await wrapper.setProps({ hostVersions: changed });
    expect((wrapper.get('[data-test="adoption-acknowledge"]').element as HTMLInputElement).checked).toBe(false);
    expect(wrapper.get('[data-test="adoption-submit"]').attributes('disabled')).toBeDefined();
    expect(wrapper.text()).toContain('a referenced action changed');
    await wrapper.get('[data-test="adoption-submit"]').trigger('click');
    expect(host.calls.some((c) => c.method === 'automation.adoption.submit')).toBe(false);
  });

  it('permits explicit re-review and new consent while the owner projection still carries the older dependency revision', async () => {
    const { host, wrapper } = await mountReview();
    await button(wrapper, 'Review saved authority').trigger('click'); await flushPromises();
    await wrapper.get('[data-test="adoption-acknowledge"]').setValue(true);
    host.source.dependencies[1]!.record!.name = 'Newly reviewed disabled branch';
    host.versions[2]!.revision = '4';
    await wrapper.get('[data-test="adoption-submit"]').trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('a referenced action changed');
    expect(host.calls.some((c) => c.method === 'automation.adoption.submit')).toBe(false);
    await button(wrapper, 'Read saved source again').trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('Newly reviewed disabled branch');
    expect(wrapper.text()).not.toContain('a referenced action changed');
    expect(wrapper.get('[data-test="adoption-acknowledge"]').attributes('disabled')).toBeUndefined();
    expect((wrapper.get('[data-test="adoption-acknowledge"]').element as HTMLInputElement).checked).toBe(false);
    await wrapper.get('[data-test="adoption-acknowledge"]').setValue(true);
    await wrapper.get('[data-test="adoption-submit"]').trigger('click'); await flushPromises();
    expect(host.calls.filter((c) => c.method === 'automation.adoption.submit')).toHaveLength(1);
    expect(host.calls.find((c) => c.method === 'automation.adoption.submit')?.params?.reviewedVersions).toEqual(host.versions);
  });

  it('shows recoverable native pending state without a selected editor and requires acknowledgement to archive uncertainty', async () => {
    const host = proposalFixture();
    host.state.pending = { commandId: 'survived-reload', kind: 'event', id: 'same', expectedRevision: '4', hostInstanceId: 'previous-instance', createdAt: 1 };
    const { wrapper } = await mountReview(host, false);
    expect(button(wrapper, 'Review saved authority')).toBeUndefined();
    await button(wrapper, 'Adoption pending').trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('survived-reload');
    expect(wrapper.text()).toContain('Select an existing saved automation');
    expect(host.calls.some((c) => c.method === 'automation.review')).toBe(false);
    await wrapper.get('[data-test="adoption-resolve"]').trigger('click'); await flushPromises();
    expect(host.calls.filter((c) => c.method === 'automation.adoption.resolve')).toHaveLength(1);
    expect(wrapper.get('[data-test="adoption-review-unknown"]').attributes('disabled')).toBeDefined();
    await wrapper.get('[data-test="adoption-unknown-acknowledge"]').setValue(true);
    await wrapper.get('[data-test="adoption-review-unknown"]').trigger('click'); await flushPromises();
    expect(host.calls.filter((c) => c.method === 'automation.adoption.review_unknown')).toHaveLength(1);
    expect(wrapper.text()).toContain('Uncertain outcome reviewed');
    expect(wrapper.text()).toContain('survived-reload');
    expect(host.calls.some((c) => c.method === 'automation.adoption.submit')).toBe(false);
  });

  it('clears source and consent when the selected saved record changes without changing journal history', async () => {
    const { wrapper } = await mountReview();
    await button(wrapper, 'Review saved authority').trigger('click'); await flushPromises();
    await wrapper.get('[data-test="adoption-acknowledge"]').setValue(true);
    await wrapper.setProps({ target: { kind: 'action', id: 'other' } });
    expect(wrapper.find('[data-test="adoption-submit"]').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('privateSource()');
    expect(wrapper.text()).toContain('Selected saved Action: other');
    expect(wrapper.text()).toContain('Host adoption journal');
  });

  it('reads a retained missing-target rejection with original expected revision zero', async () => {
    const host = proposalFixture();
    host.state.recent = [{ intent: { commandId: 'missing-target', kind: 'event', id: 'absent', expectedRevision: '0', hostInstanceId: 'instance-a', createdAt: 1 },
      outcome: 'rejected', revision: null, errorCode: 'automation_not_found', recordedAt: 2 }];
    const { wrapper } = await mountReview(host, false);
    await button(wrapper, 'Adoption history').trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('Adoption rejected');
    expect(wrapper.text()).toContain('missing-target');
    expect(wrapper.text()).toContain('automation_not_found');
    expect(wrapper.text()).not.toContain('Invalid automation');
  });

  it('drops hidden uncertainty consent when the dialog is closed and opened again', async () => {
    const host = proposalFixture();
    host.state.pending = { commandId: 'uncertain', kind: 'event', id: 'same', expectedRevision: '4', hostInstanceId: 'instance-a', createdAt: 1 };
    const { wrapper } = await mountReview(host, false);
    await button(wrapper, 'Adoption pending').trigger('click'); await flushPromises();
    await wrapper.get('[data-test="adoption-unknown-acknowledge"]').setValue(true);
    await button(wrapper, 'Close').trigger('click'); await flushPromises();
    await button(wrapper, 'Adoption pending').trigger('click'); await flushPromises();
    expect((wrapper.get('[data-test="adoption-unknown-acknowledge"]').element as HTMLInputElement).checked).toBe(false);
    expect(wrapper.get('[data-test="adoption-review-unknown"]').attributes('disabled')).toBeDefined();
  });

  it('provides Spanish acknowledgement and complete recovery labels', async () => {
    const { wrapper } = await mountReview();
    await activateLocale('es'); await nextTick();
    await button(wrapper, 'Revisar autoridad guardada').trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('no el borrador sin guardar del editor');
    expect(wrapper.text()).toContain('Acciones referenciadas (3)');
    expect(wrapper.text()).toContain('Acepto la autoridad de ejecución del propietario');
    expect(wrapper.text()).toContain('explícitos del propietario siguen disponibles');
    expect(button(wrapper, 'Adoptar con autoridad del propietario').exists()).toBe(true);
  });
});
