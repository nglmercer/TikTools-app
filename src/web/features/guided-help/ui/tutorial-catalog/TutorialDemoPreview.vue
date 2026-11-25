<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { tourRegistry } from '../../model/registry.ts';
import type { TourId } from '../../model/types.ts';

type DemoZone = 'toolbar' | 'tabs' | 'list' | 'editor' | 'preview' | 'detail' | 'footer';

const props = withDefaults(defineProps<{
  tourId: TourId;
  activeStep: number;
  src?: string;
}>(), { src: undefined });
const { t } = useI18n();

const defaultZones: readonly DemoZone[] = ['toolbar', 'list', 'editor', 'detail', 'footer'];
const zonesByScope: Record<string, readonly DemoZone[]> = {
  automations: ['toolbar', 'tabs', 'list', 'editor', 'footer'],
  connections: ['toolbar', 'editor', 'list', 'detail', 'footer'],
  feed: ['toolbar', 'preview', 'list', 'footer'],
  plugins: ['toolbar', 'list', 'detail', 'editor', 'footer'],
  overlays: ['toolbar', 'list', 'preview', 'editor', 'footer'],
  points: ['toolbar', 'tabs', 'list', 'detail', 'footer'],
  rankings: ['toolbar', 'tabs', 'list', 'detail', 'footer'],
  analytics: ['toolbar', 'preview', 'list', 'detail', 'footer'],
  diagnostics: ['toolbar', 'list', 'detail', 'footer'],
  settings: ['toolbar', 'list', 'detail', 'editor', 'footer'],
  account: ['toolbar', 'list', 'detail', 'editor', 'footer'],
};
const lineCount: Record<DemoZone, number> = { toolbar: 1, tabs: 0, list: 4, editor: 5, preview: 2, detail: 3, footer: 2 };
const blockCount: Record<DemoZone, number> = { toolbar: 2, tabs: 4, list: 0, editor: 1, preview: 1, detail: 1, footer: 2 };
const logicalWidth = 1280;
const logicalHeight = 720;

const viewport = ref<HTMLElement | null>(null);
const frame = ref<HTMLIFrameElement | null>(null);
const viewportWidth = ref(logicalWidth);
const viewportHeight = ref(logicalHeight);
const frameReady = ref(false);
let resizeObserver: ResizeObserver | undefined;
let syncGeneration = 0;

const tour = computed(() => tourRegistry[props.tourId]);
const zones = computed(() => zonesByScope[tour.value.scope] ?? defaultZones);
const target = computed(() => tour.value.steps[Math.min(props.activeStep, tour.value.steps.length - 1)]?.target ?? '');
const frameSrc = computed(() => {
  if (!props.src) return '';
  try {
    const url = new URL(props.src, window.location.href);
    return url.origin === window.location.origin ? url.href : '';
  } catch { return ''; }
});
const scale = computed(() => Math.min(1, viewportWidth.value / logicalWidth, viewportHeight.value / logicalHeight));
const frameStyle = computed(() => ({
  width: `${logicalWidth}px`,
  height: `${logicalHeight}px`,
  left: `${Math.max(0, (viewportWidth.value - logicalWidth * scale.value) / 2)}px`,
  transform: `scale(${scale.value})`,
}));

function classifyTarget(value: string): DemoZone {
  if (value.includes('tab')) return 'tabs';
  if (/(catalog|events|actions|variables|widgets|leaderboard|competitions|log|activity|contributors|list)/.test(value)) return 'list';
  if (/(profile|filter|control|library|range|search|view|sections|gateway|tools|plan)/.test(value)) return 'toolbar';
  if (/(editor|form|settings|permission|install|design|create|updater|url)/.test(value)) return 'editor';
  if (/(preview|stage|metrics|facts)/.test(value)) return 'preview';
  if (/(detail|explorer|plugin|inspector|review)/.test(value)) return 'detail';
  if (/(run|changes|history)/.test(value)) return 'footer';
  return 'list';
}

const activeZone = computed(() => {
  const zone = classifyTarget(target.value);
  return zones.value.includes(zone) ? zone : (zones.value.includes('list') ? 'list' : zones.value[0]);
});

function frameDocument(): Document | null {
  try { return frame.value?.contentDocument ?? null; }
  catch { return null; }
}
function visible(doc: Document, element: Element | null): element is HTMLElement {
  const view = doc.defaultView;
  if (!view || !(element instanceof view.HTMLElement) || !element.isConnected || element.closest('[hidden], [inert]')) return false;
  const style = view.getComputedStyle(element);
  if (style.display === 'none' || style.visibility === 'hidden' || style.visibility === 'collapse' || Number.parseFloat(style.opacity) <= 0) return false;
  return [...element.getClientRects()].some((rect) => rect.width > 0 && rect.height > 0);
}
function findVisible(doc: Document, selector: string): HTMLElement | undefined {
  try { return [...doc.querySelectorAll(selector)].find((element) => visible(doc, element)) as HTMLElement | undefined; }
  catch { return undefined; }
}
function delay(ms: number): Promise<void> {
  return new Promise((resolve) => { setTimeout(resolve, ms); });
}
async function waitForVisible(doc: Document, selector: string, generation: number, timeout = 1400): Promise<HTMLElement | undefined> {
  const deadline = Date.now() + timeout;
  while (generation === syncGeneration && Date.now() < deadline) {
    const element = findVisible(doc, selector);
    if (element) return element;
    await delay(80);
  }
  return undefined;
}
function installHighlightStyle(doc: Document): void {
  if (doc.getElementById('tutorial-demo-preview-highlight')) return;
  const style = doc.createElement('style');
  style.id = 'tutorial-demo-preview-highlight';
  style.textContent = '[data-tutorial-demo-highlight]{outline:3px solid var(--color-border-selected,#4f7cff)!important;outline-offset:3px!important;border-radius:4px}';
  doc.head.append(style);
}
function clearHighlights(doc: Document): void {
  for (const element of doc.querySelectorAll('[data-tutorial-demo-highlight]')) element.removeAttribute('data-tutorial-demo-highlight');
}
async function synchronizeFrame(): Promise<void> {
  const generation = ++syncGeneration;
  if (!frameReady.value || !frameSrc.value) return;
  const doc = frameDocument();
  const step = tour.value.steps[Math.min(props.activeStep, tour.value.steps.length - 1)];
  if (!doc || !step) return;
  clearHighlights(doc);
  const app = await waitForVisible(doc, '.app-layout', generation, 2500);
  if (!app || generation !== syncGeneration) return;
  const targetSelector = `[data-guide="${step.target}"]`;
  let element = await waitForVisible(doc, targetSelector, generation, 500);
  for (const selector of step.prepareClick ?? []) {
    if (element || generation !== syncGeneration) break;
    const control = await waitForVisible(doc, selector, generation, 700);
    if (generation !== syncGeneration) return;
    if (!control) continue;
    control.click();
    element = await waitForVisible(doc, targetSelector, generation, 500);
  }
  element ??= await waitForVisible(doc, targetSelector, generation, 1200);
  if (!element || generation !== syncGeneration) return;
  installHighlightStyle(doc);
  clearHighlights(doc);
  element.setAttribute('data-tutorial-demo-highlight', '');
  element.scrollIntoView({ block: 'center', inline: 'nearest' });
}
function onFrameLoad(): void {
  frameReady.value = true;
}

function observeViewport(element: HTMLElement | null): void {
  resizeObserver?.disconnect();
  resizeObserver = undefined;
  viewportWidth.value = element?.clientWidth || logicalWidth;
  viewportHeight.value = element?.clientHeight || logicalHeight;
  if (!element || typeof ResizeObserver === 'undefined') return;
  resizeObserver = new ResizeObserver((entries) => {
    viewportWidth.value = entries[0]?.contentRect.width || logicalWidth;
    viewportHeight.value = entries[0]?.contentRect.height || logicalHeight;
  });
  resizeObserver.observe(element);
}
onMounted(() => { observeViewport(viewport.value); });
onBeforeUnmount(() => {
  syncGeneration += 1;
  resizeObserver?.disconnect();
});
watch(viewport, observeViewport);
watch(frameSrc, () => { frameReady.value = false; });
watch([frameSrc, frameReady, () => props.activeStep], () => { void synchronizeFrame(); }, { flush: 'post', immediate: true });
</script>

<template>
  <section
    class="tutorial-demo-preview"
    :aria-label="t('guides.catalog.demoPreview')"
  >
    <div class="tutorial-demo-preview__shell">
      <header class="tutorial-demo-preview__chrome">
        <span
          class="tutorial-demo-preview__dots"
          aria-hidden="true"
        ><i /><i /><i /></span>
        <strong>{{ t('guides.catalog.demoPreview') }}</strong>
        <span class="tutorial-demo-preview__route">{{ tour.route }}</span>
      </header>
      <div
        v-if="frameSrc"
        ref="viewport"
        class="tutorial-demo-preview__real"
      >
        <iframe
          ref="frame"
          data-testid="tutorial-demo-frame"
          :src="frameSrc"
          :style="frameStyle"
          :title="`${t('guides.catalog.demoPreview')} - ${tour.route}`"
          loading="lazy"
          @load="onFrameLoad"
        />
      </div>
      <div
        v-else
        class="tutorial-demo-preview__window"
        aria-hidden="true"
      >
        <div class="tutorial-demo-preview__app">
          <aside class="tutorial-demo-preview__nav">
            <span
              v-for="item in 5"
              :key="item"
            />
          </aside>
          <main
            class="tutorial-demo-preview__canvas"
            :data-scope="tour.scope"
          >
            <div
              v-for="zone in zones"
              :key="zone"
              class="tutorial-demo-preview__zone"
              :class="[`tutorial-demo-preview__zone--${zone}`, { 'tutorial-demo-preview__zone--active': zone === activeZone }]"
            >
              <span
                v-for="line in lineCount[zone]"
                :key="`line-${line}`"
                class="tutorial-demo-preview__line"
              />
              <span
                v-for="block in blockCount[zone]"
                :key="`block-${block}`"
                class="tutorial-demo-preview__block"
              />
            </div>
          </main>
        </div>
      </div>
    </div>
  </section>
</template>
