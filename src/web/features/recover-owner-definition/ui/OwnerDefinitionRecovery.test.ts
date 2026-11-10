import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import type { DefinitionIntent } from '@/entities/automation/model/definition-contracts.ts';
import OwnerDefinitionRecovery from './OwnerDefinitionRecovery.vue';

enableAutoUnmount(afterEach);
beforeEach(() => activateLocale('en'));
afterEach(() => activateLocale('en'));
const pending: DefinitionIntent = { commandId: `owner-definition-${'a'.repeat(32)}`, operation: 'update', kind: 'action', id: 'deleted-or-unselected',
  expectedRevision: '9007199254740993', hostInstanceId: 'instance-a', recordDigest: 'c'.repeat(64), createdAt: 1 };
function fixture() {
  const native = nativeDefinitionHost(() => { throw new Error('recovery must not mutate'); });
  native.replaceState({ ...native.state(), pending });
  const transport = createFakeControlClient(native.handle);
  const wrapper = mountWithPlugins(OwnerDefinitionRecovery, { props: { selectionKey: 'action:other' }, attachTo: document.body,
    global: { provide: { [controlClientKey as symbol]: transport.client } } });
  return { native, ...transport, wrapper };
}
test('shows selection-independent original metadata and checks recovery without installing or persisting source', async () => {
  const storing = vi.spyOn(Storage.prototype, 'setItem');
  try {
    const f = fixture(); await flushPromises();
    await f.wrapper.get('[data-test="definition-history"]').trigger('click'); await flushPromises();
    expect(f.wrapper.get('[data-test="definition-pending"]').text()).toContain('deleted-or-unselected');
    expect(f.wrapper.text()).toContain('9007199254740993');
    expect(f.wrapper.text()).toContain(pending.commandId);
    f.native.replaceState({ ...f.native.state(), pending: null, recent: [{ intent: pending, outcome: 'succeeded',
      receipt: { revision: '9007199254740994', recordDigest: pending.recordDigest, enabled: false, deleted: null, affectedResources: 1 }, errorCode: null, recordedAt: 2 }] });
    await f.wrapper.get('[data-test="definition-resolve"]').trigger('click'); await flushPromises();
    expect(f.wrapper.get('[data-test="definition-receipt"]').text()).toContain('Succeeded');
    expect(f.wrapper.emitted('recovered')).toHaveLength(1);
    expect(f.calls.filter((c) => c.method === 'automation.definition.submit')).toHaveLength(0);
    expect(storing).not.toHaveBeenCalled();
    expect(f.wrapper.text()).toContain('does not replace your draft');
  } finally { storing.mockRestore(); }
});

test('unknown acknowledgement resets on selection and host changes', async () => {
  const f = fixture(); await flushPromises();
  await f.wrapper.get('[data-test="definition-history"]').trigger('click'); await flushPromises();
  await f.wrapper.get('[data-test="definition-unknown-consent"]').setValue(true);
  await f.wrapper.setProps({ selectionKey: 'event:new-selection' });
  expect((f.wrapper.get('[data-test="definition-unknown-consent"]').element as HTMLInputElement).checked).toBe(false);
  expect(f.wrapper.get('[data-test="definition-archive"]').attributes('disabled')).toBeDefined();
  await flushPromises();
  expect(f.wrapper.text()).toContain('Check original outcome');
  expect(f.wrapper.text()).toContain('does not replace your draft');
  await f.wrapper.get('[data-test="definition-unknown-consent"]').setValue(true);
  await f.wrapper.get('[data-test="definition-archive"]').trigger('click'); await flushPromises();
  expect(f.native.state().pending).toBeNull();
  expect(f.native.state().recent[0]?.outcome).toBe('reviewed-unknown');
  expect(f.calls.filter((c) => c.method === 'automation.definition.review_unknown')).toHaveLength(1);
});
