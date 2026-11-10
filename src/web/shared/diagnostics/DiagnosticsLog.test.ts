import { afterEach, expect, test, vi } from 'vitest';
import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import DiagnosticsLog from './DiagnosticsLog.vue';
import { diagnosticLog, diagnosticView } from './log.ts';
enableAutoUnmount(afterEach);
afterEach(() => { diagnosticLog.clear(); diagnosticView.source.value = ''; diagnosticView.entityId.value = ''; diagnosticView.selectedId.value = ''; vi.restoreAllMocks(); });
test('shows the contextual entry once, including an older selection beyond the recent page', () => {
  const first = diagnosticLog.record({ source: 'plugins', entityId: 'game', level: 'error', code: 'test.error', message: 'selected failure', details: 'safe technical details' });
  for (let i = 0; i < 105; i++) diagnosticLog.record({ source: 'plugins', entityId: 'game', level: 'info', code: 'info', message: `message ${i}` });
  diagnosticView.selectedId.value = first.id;
  const wrapper = mountWithPlugins(DiagnosticsLog);
  expect(wrapper.findAll('.logs__message').filter(row => row.text() === 'selected failure')).toHaveLength(1);
  expect(wrapper.get('.logs__row--selected').text()).toContain('safe technical details');
  expect(wrapper.get('.logs__row--selected button').attributes('aria-expanded')).toBe('true');
});
test('changing source clears entity context so filters do not hide unrelated sources', async () => {
  diagnosticLog.record({ source: 'plugins', entityId: 'game', level: 'error', code: 'failed', message: 'plugin failure' });
  diagnosticLog.record({ source: 'sync', level: 'error', code: 'failed', message: 'sync failure' });
  diagnosticView.source.value = 'plugins'; diagnosticView.entityId.value = 'game';
  const wrapper = mountWithPlugins(DiagnosticsLog);
  wrapper.findAllComponents(BaseComboBox)[0]!.vm.$emit('update:modelValue', 'sync');
  await flushPromises();
  expect(diagnosticView.entityId.value).toBe('');
  expect(wrapper.findAll('.logs__message').map(row => row.text())).toEqual(['sync failure']);
});
test('copies redacted selected details and clearing history preserves domain-independent empty state', async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.spyOn(navigator.clipboard, 'writeText').mockImplementation(writeText);
  const entry = diagnosticLog.record({ source: 'plugins', entityId: 'game', level: 'error', code: 'failed', message: 'password=private-secret', details: 'token=another-secret' });
  diagnosticView.selectedId.value = entry.id;
  const wrapper = mountWithPlugins(DiagnosticsLog);
  await wrapper.findAll('button').find(button => button.text() === 'Copy entry')!.trigger('click');
  expect(writeText).toHaveBeenCalledOnce();
  expect(writeText.mock.calls[0]![0]).not.toContain('private-secret');
  expect(writeText.mock.calls[0]![0]).not.toContain('another-secret');
  await wrapper.get('button[aria-label="Clear history"]').trigger('click');
  expect(wrapper.text()).toContain('No entries yet.');
});

test('filters collapse without losing state and copy scopes stay distinct', async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.spyOn(navigator.clipboard, 'writeText').mockImplementation(writeText);
  diagnosticLog.record({ source: 'plugins', level: 'error', code: 'one', message: 'plugin failure' });
  diagnosticLog.record({ source: 'sync', level: 'info', code: 'two', message: 'sync complete' });
  const wrapper = mountWithPlugins(DiagnosticsLog, { attachTo: document.body });
  const toggle = wrapper.get('.logs__filter-button');
  expect(toggle.attributes('aria-expanded')).toBe('false');
  await toggle.trigger('click');
  wrapper.findAllComponents(BaseComboBox)[0]!.vm.$emit('update:modelValue', 'plugins');
  await flushPromises();
  await wrapper.get('button[aria-label="Copy filtered"]').trigger('click');
  expect(writeText.mock.calls.at(-1)?.[0]).toContain('plugin failure');
  expect(writeText.mock.calls.at(-1)?.[0]).not.toContain('sync complete');
  await wrapper.findAll('button').find(button => button.text() === 'Copy all')!.trigger('click');
  expect(writeText.mock.calls.at(-1)?.[0]).toContain('sync complete');
  await wrapper.get('.logs__filters').trigger('keydown', { key: 'Escape' });
  expect(toggle.attributes('aria-expanded')).toBe('false');
  expect(document.activeElement).toBe(toggle.element);
  expect(wrapper.get('.logs__chip').text()).toContain('Plugins');
  expect(wrapper.findAll('.logs__message')).toHaveLength(1);
  await wrapper.get('.logs__chip').trigger('click');
  expect(wrapper.findAll('.logs__message')).toHaveLength(2);
});
test('does not repeat identical details but preserves the full export text', async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.spyOn(navigator.clipboard, 'writeText').mockImplementation(writeText);
  const entry = diagnosticLog.record({ source: 'updates', level: 'info', code: 'unavailable', message: 'Not supported', details: 'Not supported' });
  diagnosticView.selectedId.value = entry.id;
  const wrapper = mountWithPlugins(DiagnosticsLog);
  expect(wrapper.find('pre').exists()).toBe(false);
  await wrapper.findAll('button').find(button => button.text() === 'Copy entry')!.trigger('click');
  expect(writeText.mock.calls[0]?.[0]).toContain('Not supported');
});
test('keeps clipboard fallback redacted when clipboard access fails', async () => {
  vi.spyOn(navigator.clipboard, 'writeText').mockRejectedValue(new Error('unavailable'));
  diagnosticLog.record({ source: 'account', level: 'error', code: 'failed', message: 'password=private-secret' });
  const wrapper = mountWithPlugins(DiagnosticsLog);
  await wrapper.get('button[aria-label="Copy all"]').trigger('click');
  expect((wrapper.get('textarea').element as HTMLTextAreaElement).value).not.toContain('private-secret');
  expect(wrapper.get('[role=status]').text()).toContain('Clipboard unavailable');
});

test('offers only one visible reset when a selected entry is filtered out', async () => {
  const entry = diagnosticLog.record({ source: 'updates', level: 'info', code: 'unavailable', message: 'Unavailable' });
  diagnosticView.selectedId.value = entry.id;
  const wrapper = mountWithPlugins(DiagnosticsLog, { attachTo: document.body });
  await wrapper.get('input[type=search]').setValue('unmatched');
  const resets = () => wrapper.findAll('button').filter(button => button.text() === 'Clear filters' && button.isVisible());
  expect(resets()).toHaveLength(1);
  await wrapper.get('.logs__filter-button').trigger('click');
  expect(resets()).toHaveLength(1);
});
