import { storeToRefs } from 'pinia';
import { onMounted, onUnmounted, ref, watch } from 'vue';
import { parsePluginNavId } from '../../../automation/plugins/declarative.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { filterEvents } from '@/entities/live-event/lib/event-filter.ts';
import { emoteImageFailureCount } from '@/entities/live-event/lib/emote-failures.ts';
import { useLiveFeedStore } from '@/entities/live-event/model/live-feed-store.ts';
import { useDiscoveryStore } from '@/entities/live-room/model/discovery-store.ts';
import { pluginHostDiagnostics } from '@/entities/plugin/api/plugin-webview-host.ts';
import { usePluginsStore } from '@/entities/plugin/model/plugins-store.ts';
import { usePointsStore } from '@/entities/points/model/points-store.ts';
import { useProcessorsStore } from '@/entities/processor/model/processors-store.ts';
import { useRuleProfilesStore } from '@/entities/rule-template/model/rule-profiles-store.ts';
import { useGlobalsStore } from '@/entities/runtime-global/model/globals-store.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useNavigationStore } from '@/shared/model/navigation-store.ts';
import type { AppTab } from '@/types.ts';

const FRAME_SAMPLES = 240;

export function useAppRuntime() {
  const control = useControlClient();
  const navigation = useNavigationStore();
  const { activeTab } = storeToRefs(navigation);
  const live = useLiveFeedStore();
  const points = usePointsStore();
  const automation = useAutomationStore();
  const connection = useConnectionStore();
  const plugins = usePluginsStore();
  const processors = useProcessorsStore();
  const globals = useGlobalsStore();
  const profiles = useRuleProfilesStore();
  const discovery = useDiscoveryStore();
  const { events, autoScroll, filter, searchQuery } = storeToRefs(live);

  let disposed = false;
  let presentationMounted = false;
  let resyncInFlight = false;
  let resyncAgain = false;

  function resync(): void {
    if (disposed) return;
    if (resyncInFlight) {
      resyncAgain = true;
      return;
    }
    resyncInFlight = true;
    resyncAgain = false;
    void Promise.allSettled([
      processors.refresh(),
      connection.refreshStatus(),
      connection.refreshMonitor(),
      points.refresh(),
      connection.refreshCreators(),
      automation.refresh(),
      live.refresh(),
      globals.load(),
      profiles.loadProfiles(),
    ]).finally(() => {
      resyncInFlight = false;
      if (resyncAgain) resync();
    });
  }
  const offGap = control.onGap(resync);

  watch([events, autoScroll, activeTab, filter, searchQuery], () => {
    if (activeTab.value === 'feed' && autoScroll.value) live.scrollToBottom();
  });

  watch(
    () => [automation.pluginPages, activeTab.value] as const,
    ([pages, tab]) => {
      const parsed = parsePluginNavId(tab);
      if (parsed && !pages.some((page) => page.pluginId === parsed.pluginId && page.id === parsed.pageId)) {
        navigation.open('plugins');
      }
    },
  );

  const pageVisible = ref(document.visibilityState !== 'hidden');
  const hostVisible = ref(true);
  const offHostVisibility = control.onTopic<{ visible: boolean }>('ui.host.visibility', (data) => {
    if (typeof data.visible === 'boolean') hostVisible.value = data.visible;
  });

  function applyPresentation(visible: boolean, tab: AppTab): void {
    const feed = tab === 'feed';
    live.setPresentationActive(visible && feed);
    points.setPresentationActive(visible && (feed || tab === 'points'));
    automation.setPresentationActive(visible && tab === 'behavior');
  }

  function updatePresentation(): void {
    const visible = pageVisible.value && hostVisible.value;
    const tab = activeTab.value;
    applyPresentation(visible, tab);
    if (!presentationMounted || disposed) return;
    const feed = tab === 'feed';
    void control
      .setPresentation?.({ visible, feed, points: feed || tab === 'points', behavior: tab === 'behavior' })
      .catch((failure: unknown) => {
        if (disposed || (failure instanceof Error && 'code' in failure && failure.code === 'stale')) return;
        console.warn(`presentation handshake failed: ${errorMessage(failure)}`);
      });
    if (visible && (feed || tab === 'points')) void points.refresh();
    if (visible && tab === 'behavior') void automation.refresh();
  }

  function onVisibilityChange(): void {
    pageVisible.value = document.visibilityState !== 'hidden';
  }

  watch([activeTab, pageVisible, hostVisible], updatePresentation, { flush: 'sync' });
  applyPresentation(pageVisible.value, activeTab.value);

  const diagnosticsEnabled = import.meta.env?.DEV || import.meta.env?.VITE_TIKTOOLS_DIAGNOSTICS === '1';
  const frameIntervals: number[] = [];
  let diagnosticFrame = 0;
  let lastFrame: number | null = null;
  let measuredFrames = 0;

  function measureFrame(at: number): void {
    if (disposed) return;
    if (lastFrame !== null) {
      frameIntervals.push(at - lastFrame);
      if (frameIntervals.length > FRAME_SAMPLES) frameIntervals.shift();
    }
    lastFrame = at;
    measuredFrames += 1;
    diagnosticFrame = requestAnimationFrame(measureFrame);
  }

  watch(
    [pageVisible, hostVisible],
    () => {
      if (!diagnosticsEnabled || typeof requestAnimationFrame !== 'function') return;
      cancelAnimationFrame(diagnosticFrame);
      diagnosticFrame = 0;
      lastFrame = null;
      if (pageVisible.value && hostVisible.value) diagnosticFrame = requestAnimationFrame(measureFrame);
    },
    { immediate: true },
  );

  function diagnostics(): Record<string, unknown> {
    const memory = (performance as Performance & { memory?: { usedJSHeapSize?: number } }).memory;
    const images = Array.from(document.images);
    const intervals = [...frameIntervals].sort((left, right) => left - right);
    const percentile = (quantile: number): number | null =>
      intervals.length > 0 ? (intervals[Math.min(intervals.length - 1, Math.floor(intervals.length * quantile))] ?? null) : null;
    return {
      matchingFeedEvents: filterEvents(events.value, filter.value, searchQuery.value).length,
      emoteImageFailures: emoteImageFailureCount(),
      liveMonitor: connection.monitorStatus,
      renderedFeedRows: document.querySelectorAll('.event-row').length,
      frameCallbacks: measuredFrames,
      frameIntervalSamples: intervals.length,
      frameIntervalP95Ms: percentile(0.95),
      frameIntervalP99Ms: percentile(0.99),
      pageVisibility: document.visibilityState,
      domNodes: document.querySelectorAll('*').length,
      imageCount: images.length,
      loadedImages: images.filter((image) => image.complete && image.naturalWidth > 0).length,
      imagePixelBytesEstimate: images.reduce((sum, image) => sum + image.naturalWidth * image.naturalHeight * 4, 0),
      control: control.diagnostics?.(),
      live: live.diagnostics(),
      ownedTimers: plugins.diagnostics().timers + automation.diagnostics().timers + live.diagnostics().timers,
      pluginHosts: pluginHostDiagnostics(),
      iframeCount: document.querySelectorAll('iframe').length,
      jsHeapBytes: memory?.usedJSHeapSize ?? null,
    };
  }

  const benchmarkControls = {
    setTab: (tab: AppTab) => navigation.open(tab),
    setImagesEnabled: (enabled: boolean) => {
      live.imagesEnabled = enabled;
    },
    setRenderLimit: (limit: number) => {
      live.renderLimit = Math.max(50, Math.min(300, Math.floor(limit)));
    },
  };
  const scope = window as unknown as Record<string, unknown>;
  if (diagnosticsEnabled) {
    scope['__tiktools_frontend_diagnostics__'] = diagnostics;
    scope['__tiktools_benchmark_controls__'] = benchmarkControls;
  }

  onMounted(() => {
    control.attach();
    presentationMounted = true;
    document.addEventListener('visibilitychange', onVisibilityChange);
    updatePresentation();
    void connection.refreshStatus();
    void connection.refreshMonitor();
    if (activeTab.value !== 'feed' && activeTab.value !== 'points') void points.refresh();
    void connection.refreshCreators();
    if (activeTab.value !== 'behavior') void automation.refresh();
    void live.refresh();
    void processors.refresh();
    void globals.load();
    void profiles.loadProfiles();
  });

  onUnmounted(() => {
    presentationMounted = false;
    disposed = true;
    offGap();
    offHostVisibility();
    document.removeEventListener('visibilitychange', onVisibilityChange);
    void control.setPresentation?.({ visible: false, feed: false, points: false, behavior: false }).catch(() => {});
    if (diagnosticFrame !== 0) cancelAnimationFrame(diagnosticFrame);
    frameIntervals.length = 0;
    if (scope['__tiktools_frontend_diagnostics__'] === diagnostics) delete scope['__tiktools_frontend_diagnostics__'];
    if (scope['__tiktools_benchmark_controls__'] === benchmarkControls) delete scope['__tiktools_benchmark_controls__'];
    plugins.dispose();
    automation.dispose();
    connection.dispose();
    discovery.dispose();
    live.dispose();
    points.dispose();
    control.detach();
  });
}
