import { inject, nextTick } from 'vue';
import { routerKey } from 'vue-router';
import { diagnosticLog, diagnosticView, type DiagnosticSource } from './log.ts';

let returnFocus: HTMLElement | null = null;
export function useDiagnosticNavigation() {
  const router = inject(routerKey, null);
  function open(source?: DiagnosticSource, entityId = ''): void {
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    diagnosticView.source.value = source ?? '';
    diagnosticView.entityId.value = entityId;
    diagnosticView.selectedId.value = source ? diagnosticLog.latest(source, entityId)?.id ?? '' : '';
    diagnosticLog.markRead();
    diagnosticView.request.value++;
    if (window.matchMedia('(max-width: 767px)').matches && router) {
      diagnosticView.open.value = false;
      const from = router.currentRoute.value;
      void router.push({ path: '/debug', query: {
        source: source ?? '', entity: entityId, entry: diagnosticView.selectedId.value,
        returnTo: from.path === '/debug' ? String(from.query.returnTo ?? '/connections') : from.fullPath,
      } });
    } else diagnosticView.open.value = true;
  }
  async function close(): Promise<void> {
    diagnosticView.open.value = false;
    await nextTick();
    if (returnFocus?.isConnected) returnFocus.focus();
  }
  return { open, close };
}
