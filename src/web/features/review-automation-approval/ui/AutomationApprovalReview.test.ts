import { enableAutoUnmount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { approvalFixture, nativeId } from '../testing/fixture.ts';
import AutomationApprovalReview from './AutomationApprovalReview.vue';

enableAutoUnmount(afterEach);
beforeEach(() => activateLocale('en'));
afterEach(() => activateLocale('en'));
const button = (wrapper: VueWrapper, label: string) => wrapper.findAll('button').find((node) => node.text() === label)!;
async function mountReview(host = approvalFixture(), selected = true) {
  const wrapper = mountWithPlugins(AutomationApprovalReview, { attachTo: document.body,
    props: { target: selected ? { kind: 'event', id: 'same' } : null, hostVersions: host.projection() },
    global: { provide: { [controlClientKey as symbol]: host.client } } });
  await flushPromises();
  return { host, wrapper };
}
describe('owner delegated execution approval UI', () => {
  it('labels scoped custom triggers and opaque script data flow', async () => {
    const host = approvalFixture();
    host.source.record!['trigger'] = 'moderator.alert';
    host.source.dependencies[0]!.record!['typeId'] = 'core.code';
    host.source.dependencies[0]!.record!['config'] = { source: 'return { log: [event.user.uniqueId] };' };
    host.subjects[0]!.grant.capabilities.push('scripts.execute', 'events.emit');
    host.subjects[0]!.grant.effects = { http: [], audio: [], emitTopics: ['moderator.alert'] };
    host.overrides.set('automation.approval.review', (params, options) => {
      const result = host.handle('automation.approval.review', params, options) as ReturnType<typeof host.review>;
      Object.assign(result.manifest!.effects[0]!, { kind: 'script', decision: 'review-required', renderFields: [], dynamicTarget: true, requiredCapabilities: ['scripts.execute', 'live.read'], reasons: ['opaque-script'] });
      result.manifest!.reasons.push('opaque-script');
      return result;
    });
    const { wrapper } = await mountReview(host);
    await wrapper.get('[data-test="approval-open"]').trigger('click'); await flushPromises();
    expect(wrapper.get('[data-test="approval-scoped-trigger"]').text()).toContain('moderator.alert');
    expect(wrapper.get('[data-test="approval-scoped-trigger"]').text()).toContain('already approved delegated execution');
    expect(wrapper.get('[data-test="approval-script-policy"]').text()).toContain('all public input fields in one HTTP rule');
    await nextTick();
    expect(wrapper.get('[data-test="approval-scoped-trigger"]').text()).toContain('an already approved delegated execution');
    expect(wrapper.get('[data-test="approval-script-policy"]').text()).toContain('all public input fields in one HTTP rule');
  });
  it('shows one explicitly read saved closure, escaped source, exact effects, field pins and author restrictions', async () => {
    const { host, wrapper } = await mountReview();
    expect(host.calls.some((c) => c.method === 'automation.approval.review')).toBe(false);
    await wrapper.get('[data-test="approval-open"]').trigger('click'); await flushPromises();
    expect(wrapper.get('[data-test="approval-source"]').text()).toContain('Saved delegated proposal');
    expect(wrapper.get('[data-test="approval-dependency-source-disabled"]').text()).toContain('Disabled alternative');
    expect(wrapper.get('[data-test="approval-dependency-source-same"]').text()).toContain('<script>privateSource()</script>');
    expect(wrapper.find('script').exists()).toBe(false);
    expect(wrapper.findAll('[data-test="approval-effect"]')).toHaveLength(3);
    expect(wrapper.get('[data-test="approval-manifest"]').text()).toContain('event.user.uniqueId');
    expect(wrapper.get('[data-test="approval-subject"]').text()).toContain('Host configuration revision: 2');
    expect(wrapper.get('[data-test="approval-subject"]').text()).toContain('"maxPointsDelta": 5');
    expect(wrapper.get('[data-test="approval-submit"]').attributes('disabled')).toBeDefined();
    expect(wrapper.get('[data-test="approval-enable-plan"]').text()).toContain('Owner action: change its enabled state separately');
    expect(wrapper.find('[data-test="approval-plan-include-owner"]').exists()).toBe(false);
  });

  it('requires renewed consent after explicit dependency state changes and sends only native intent parameters', async () => {
    const write = vi.spyOn(Storage.prototype, 'setItem');
    const { host, wrapper } = await mountReview();
    await wrapper.get('[data-test="approval-open"]').trigger('click'); await flushPromises();
    await wrapper.get('[data-test="approval-acknowledge"]').setValue(true);
    await wrapper.get('[data-test="approval-plan-include-disabled"]').setValue(true);
    expect((wrapper.get('[data-test="approval-acknowledge"]').element as HTMLInputElement).checked).toBe(false);
    await wrapper.get('[data-test="approval-acknowledge"]').setValue(true);
    await wrapper.get('[data-test="approval-submit"]').trigger('click'); await flushPromises();
    expect(host.calls.filter((c) => c.method === 'automation.approval.submit')).toHaveLength(1);
    const request = host.requests.find((c) => c.method === 'automation.approval.submit')!;
    expect(request.params?.enablePlan).toEqual([{ kind: 'event', id: 'same', enabled: true }, { kind: 'action', id: 'disabled', enabled: true }]);
    expect(request.params).not.toHaveProperty('commandId');
    expect(request.params).not.toHaveProperty('source');
    expect(request.options?.meta).toBeUndefined();
    expect(wrapper.get('[data-test="approval-journal"]').text()).toContain(nativeId());
    expect(wrapper.get('[data-test="approval-journal"]').text()).toContain('Operation confirmed');
    expect(wrapper.text()).toContain('This review shows the earlier saved state');
    expect(write).not.toHaveBeenCalled();
  });

  it('keeps unsupported or missing references inspectable and cannot acknowledge them into eligibility', async () => {
    const host = approvalFixture();
    host.source.dependencies[2]!.record = null;
    host.eligibility = { eligible: false, reasonCode: 'policy-unavailable' };
    const { wrapper } = await mountReview(host);
    await wrapper.get('[data-test="approval-open"]').trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('No saved record exists at this ID.');
    expect(wrapper.text()).toContain('Acknowledgement cannot override this');
    expect(wrapper.get('[data-test="approval-acknowledge"]').attributes('disabled')).toBeDefined();
    expect(wrapper.get('[data-test="approval-submit"]').attributes('disabled')).toBeDefined();
    await wrapper.get('[data-test="approval-submit"]').trigger('click');
    expect(host.calls.some((c) => c.method === 'automation.approval.submit')).toBe(false);
  });

  it('allows checked revocation even when an author is unavailable and no current manifest can be resolved', async () => {
    const host = approvalFixture();
    host.manifestAvailable = false;
    host.eligibility = { eligible: false, reasonCode: 'subjects-unavailable' };
    host.approval = { state: 'active', generation: '3', policyVersion: 1, manifestDigest: 'c'.repeat(64), current: false };
    const { wrapper } = await mountReview(host);
    await wrapper.get('[data-test="approval-open"]').trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('No current manifest can be resolved');
    expect(wrapper.get('[data-test="approval-submit"]').attributes('disabled')).toBeDefined();
    expect(wrapper.get('[data-test="approval-revoke"]').attributes('disabled')).toBeDefined();
    await wrapper.get('[data-test="approval-revoke-acknowledge"]').setValue(true);
    expect(wrapper.get('[data-test="approval-revoke"]').attributes('disabled')).toBeUndefined();
    await wrapper.get('[data-test="approval-revoke"]').trigger('click'); await flushPromises();
    expect(host.calls.find((c) => c.method === 'automation.approval.submit')?.params).toEqual({ operation: 'revoke', kind: 'event', id: 'same', expectedRevision: '4', hostId: 'host-a', hostInstanceId: 'instance-a', acknowledgeRevocation: true });
  });

  it('warns about rounded source integers and disables approval consent without hiding source or retained revocation', async () => {
    const host = approvalFixture();
    host.source.record!.filters = JSON.parse('[{"field":"event.data.count","value":9007199254740993}]') as unknown[];
    host.approval = { state: 'active', generation: '2', policyVersion: 1, manifestDigest: 'b'.repeat(64), current: false };
    const { wrapper } = await mountReview(host);
    await wrapper.get('[data-test="approval-open"]').trigger('click'); await flushPromises();
    expect(wrapper.get('[data-test="approval-source-precision"]').text()).toContain('cannot display every saved integer exactly');
    expect(wrapper.get('[data-test="approval-source"]').text()).toContain('Saved delegated proposal');
    expect(wrapper.get('[data-test="approval-acknowledge"]').attributes('disabled')).toBeDefined();
    expect(wrapper.get('[data-test="approval-submit"]').attributes('disabled')).toBeDefined();
    expect(wrapper.get('[data-test="approval-revoke-acknowledge"]').attributes('disabled')).toBeUndefined();
    await wrapper.get('[data-test="approval-revoke-acknowledge"]').setValue(true);
    expect(wrapper.get('[data-test="approval-revoke"]').attributes('disabled')).toBeUndefined();
    await nextTick();
    expect(wrapper.get('[data-test="approval-source-precision"]').text()).toContain('rounded value');
  });

  it('clears consent after a referenced revision changes while retaining the reviewed source and plan', async () => {
    const { host, wrapper } = await mountReview();
    await wrapper.get('[data-test="approval-open"]').trigger('click'); await flushPromises();
    await wrapper.get('[data-test="approval-plan-include-disabled"]').setValue(true);
    await wrapper.get('[data-test="approval-acknowledge"]').setValue(true);
    const changed = host.projection(); changed.set('action:disabled', '4');
    await wrapper.setProps({ hostVersions: changed });
    expect(wrapper.text()).toContain('permissions or approval changed');
    expect(wrapper.get('[data-test="approval-source"]').text()).toContain('Saved delegated proposal');
    expect((wrapper.get('[data-test="approval-plan-include-disabled"]').element as HTMLInputElement).checked).toBe(true);
    expect((wrapper.get('[data-test="approval-acknowledge"]').element as HTMLInputElement).checked).toBe(false);
    expect(wrapper.get('[data-test="approval-submit"]').attributes('disabled')).toBeDefined();
  });

  it('recovers an original pending operation without a selection, never resends and requires explicit unknown-outcome review', async () => {
    const host = approvalFixture();
    host.state.pending = { commandId: nativeId(), operation: 'activate', kind: 'event', id: 'same', expectedRevision: '4', hostInstanceId: 'old-instance', createdAt: 1 };
    const { wrapper } = await mountReview(host, false);
    expect(wrapper.find('[data-test="approval-open"]').exists()).toBe(false);
    await wrapper.get('[data-test="approval-history"]').trigger('click'); await flushPromises();
    expect(wrapper.get('[data-test="approval-pending"]').text()).toContain(nativeId());
    expect(wrapper.text()).toContain('never submits approval or revocation again');
    await wrapper.get('[data-test="approval-resolve"]').trigger('click'); await flushPromises();
    expect(host.calls.filter((c) => c.method === 'automation.approval.resolve')).toHaveLength(1);
    expect(wrapper.get('[data-test="approval-review-unknown"]').attributes('disabled')).toBeDefined();
    await wrapper.get('[data-test="approval-unknown-acknowledge"]').setValue(true);
    await wrapper.get('[data-test="approval-review-unknown"]').trigger('click'); await flushPromises();
    expect(host.calls.filter((c) => c.method === 'automation.approval.review_unknown')).toHaveLength(1);
    expect(wrapper.text()).toContain('Uncertain outcome reviewed');
    expect(host.calls.some((c) => c.method === 'automation.approval.submit' || c.method === 'automation.approval.review')).toBe(false);
  });

  it('clears hidden uncertainty consent after closing', async () => {
    const host = approvalFixture();
    host.state.pending = { commandId: nativeId(), operation: 'revoke', kind: 'event', id: 'same', expectedRevision: '4', hostInstanceId: 'instance-a', createdAt: 1 };
    const { wrapper } = await mountReview(host, false);
    await wrapper.get('[data-test="approval-history"]').trigger('click'); await flushPromises();
    await wrapper.get('[data-test="approval-unknown-acknowledge"]').setValue(true);
    await button(wrapper, 'Close').trigger('click'); await flushPromises();
    await wrapper.get('[data-test="approval-history"]').trigger('click'); await flushPromises();
    expect((wrapper.get('[data-test="approval-unknown-acknowledge"]').element as HTMLInputElement).checked).toBe(false);
    await nextTick();
    expect(wrapper.text()).toContain('Saved approval activity');
    expect(wrapper.text()).toContain('preserves each author’s permissions');
    expect(button(wrapper, 'Resolve original outcome').exists()).toBe(true);
    expect(button(wrapper, 'Archive reviewed uncertainty').attributes('disabled')).toBeDefined();
  });
});
