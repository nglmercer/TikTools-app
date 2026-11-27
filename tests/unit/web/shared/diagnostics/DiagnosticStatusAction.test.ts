import { testMessage } from '@/shared/i18n/test-message.ts';
import { afterEach, expect, test, vi } from 'vitest';
import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import DiagnosticStatusAction from '../../../../../src/web/shared/diagnostics/DiagnosticStatusAction.vue';
import { diagnosticLog, diagnosticView } from '../../../../../src/web/shared/diagnostics/log.ts';
import { mediaQueryList } from '../../../support/media-query.ts';
enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);
afterEach(() => {
  diagnosticLog.clear();
  diagnosticView.open.value = false;
  vi.useRealTimers();
  vi.restoreAllMocks();
});
test('opens the exact plugin event without recording another entry', async () => {
  vi.spyOn(window, 'matchMedia').mockReturnValue(mediaQueryList(false));
  diagnosticLog.record({ source:'plugins', entityId:'other', code:'failed', message:'other failure', level:'error' });
  const entry = diagnosticLog.record({ source:'plugins', entityId:'game', code:'failed', message:'game failure', level:'error' });
  // Fixture label supplied by the caller, not a translated catalog string.
  const wrapper = mountWithPlugins(DiagnosticStatusAction, { props:{ source:'plugins', entityId:'game', label:'Connection failed' } });
  expect(wrapper.get('button').attributes('aria-label')).toContain(t('diagnostics.viewLogs'));
  await wrapper.get('button').trigger('click');
  expect(diagnosticView.open.value).toBe(true);
  expect(diagnosticView.selectedId.value).toBe(entry.id);
  expect(diagnosticLog.entries.value).toHaveLength(2);
});
test('keeps the status and view-logs action visible and explains the latest error on hover', async () => {
  vi.useFakeTimers();
  vi.spyOn(window, 'matchMedia').mockReturnValue(mediaQueryList(false));
  diagnosticLog.record({ source: 'live', level: 'error', code: 'offline', message: 'first failure' });
  diagnosticLog.record({ source: 'live', level: 'info', code: 'connected', message: 'connected' });
  diagnosticLog.record({ source: 'live', level: 'error', code: 'auth', message: 'latest failure' });
  const wrapper = mountWithPlugins(DiagnosticStatusAction, { props: { source: 'live', label: 'Connection failed' }, attachTo: document.body });
  const button = wrapper.get('button');
  expect(button.text()).toContain('Connection failed');
  expect(button.text()).toContain(t('diagnostics.viewLogs'));
  await button.trigger('pointerenter');
  vi.advanceTimersByTime(600);
  expect(document.getElementById('app-tooltip')?.textContent).toBe('latest failure');
});

test('mobile navigates to diagnostics with context and a return location', async () => {
  vi.spyOn(window, 'matchMedia').mockReturnValue(mediaQueryList(true));
  const router = createRouter({ history:createMemoryHistory(), routes:[{ path:'/:pathMatch(.*)*', component:{ template:'<div />' } }] });
  await router.push('/connections');
  const wrapper = mountWithPlugins(DiagnosticStatusAction, {
    props:{ source:'plugins', entityId:'game', label:'Connection failed' },
    global:{ plugins:[router] },
  });
  await wrapper.get('button').trigger('click');
  await flushPromises();
  expect(router.currentRoute.value.path).toBe('/debug');
  expect(router.currentRoute.value.query).toMatchObject({ source:'plugins', entity:'game', returnTo:'/connections' });
});
