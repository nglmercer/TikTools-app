import { afterEach, expect, test, vi } from 'vitest';
import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { createMemoryHistory, createRouter } from 'vue-router';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import DiagnosticStatusAction from './DiagnosticStatusAction.vue';
import { diagnosticLog, diagnosticView } from './log.ts';
enableAutoUnmount(afterEach);
afterEach(() => { diagnosticLog.clear(); diagnosticView.open.value = false; vi.restoreAllMocks(); });
test('opens the exact plugin event without recording another entry', async () => {
  vi.spyOn(window, 'matchMedia').mockReturnValue({ matches: false } as MediaQueryList);
  diagnosticLog.record({ source:'plugins', entityId:'other', code:'failed', message:'other failure', level:'error' });
  const entry = diagnosticLog.record({ source:'plugins', entityId:'game', code:'failed', message:'game failure', level:'error' });
  const wrapper = mountWithPlugins(DiagnosticStatusAction, { props:{ source:'plugins', entityId:'game', label:'Connection failed' } });
  expect(wrapper.get('button').attributes('aria-label')).toContain('View logs');
  await wrapper.get('button').trigger('click');
  expect(diagnosticView.open.value).toBe(true);
  expect(diagnosticView.selectedId.value).toBe(entry.id);
  expect(diagnosticLog.entries.value).toHaveLength(2);
});
test('mobile navigates to diagnostics with context and a return location', async () => {
  vi.spyOn(window, 'matchMedia').mockReturnValue({ matches: true } as MediaQueryList);
  const router = createRouter({ history:createMemoryHistory(), routes:[{ path:'/:pathMatch(.*)*', component:{ template:'<div />' } }] });
  await router.push('/connections');
  const wrapper = mountWithPlugins(DiagnosticStatusAction, { props:{ source:'plugins', entityId:'game', label:'Connection failed' }, global:{ plugins:[router] } });
  await wrapper.get('button').trigger('click'); await flushPromises();
  expect(router.currentRoute.value.path).toBe('/debug');
  expect(router.currentRoute.value.query).toMatchObject({ source:'plugins', entity:'game', returnTo:'/connections' });
});
