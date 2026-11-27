<script setup lang="ts" generic="T extends string | number">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, useId, watch } from 'vue';
import { enter } from '@/shared/lib/motion.ts';
import { firstEnabledIndex, lastEnabledIndex, stepEnabledIndex } from '@/shared/lib/list-navigation.ts';
import type { TabOption } from './tab-types.ts';

const props = defineProps<{
  tabs: readonly TabOption<T>[];
  /** Accessible name of the tab list, when the surrounding heading does not already name it. */
  label?: string;
  /**
   * Render the tab list after the #bar slot content instead of before it —
   * for headers that merge navigation and controls into a single row with
   * the tabs closing the line.
   */
  listEnd?: boolean;
}>();

const model = defineModel<T>({ required: true });
const id = useId();
const tabElements = ref<HTMLElement[]>([]);

const selectedIndex = computed(() => props.tabs.findIndex((tab) => tab.value === model.value));
const tabId = (index: number): string => `${id}-tab-${index}`;
const panelId = `${id}-panel`;

// The accent line under the selected tab is one element that slides between tabs, so a change reads as movement.
const listElement = ref<HTMLElement | null>(null);
const panelElement = ref<HTMLElement | null>(null);
const indicator = ref<{ x: number; y: number; width: number } | null>(null);
const indicatorAnimated = ref(false);
let resizeObserver: ResizeObserver | undefined;

function measureIndicator(): void {
  const tab = tabElements.value[selectedIndex.value];
  indicator.value = tab && tab.offsetWidth > 0 ? { x: tab.offsetLeft, y: tab.offsetTop + tab.offsetHeight - 1, width: tab.offsetWidth } : null;
}

onMounted(() => {
  measureIndicator();
  // Turn the slide on only after the first placement, so the line does not travel in from the corner.
  requestAnimationFrame(() => { indicatorAnimated.value = true; });
  if (typeof ResizeObserver === 'function' && listElement.value) {
    resizeObserver = new ResizeObserver(measureIndicator);
    resizeObserver.observe(listElement.value);
  }
});
onBeforeUnmount(() => resizeObserver?.disconnect());
watch(selectedIndex, async () => {
  await nextTick();
  measureIndicator();
  // Fade the new content in without remounting it, so the panel keeps the state it already had.
  if (panelElement.value) enter(panelElement.value, { shift: 4 });
});
watch(() => props.tabs.length, () => nextTick(measureIndicator));

function select(index: number): void {
  const tab = props.tabs[index];
  if (!tab || tab.disabled) return;
  model.value = tab.value;
  void nextTick(() => tabElements.value[index]?.focus());
}

function onKeydown(event: KeyboardEvent): void {
  const from = selectedIndex.value;
  const targets: Record<string, () => number> = {
    ArrowRight: () => stepEnabledIndex(props.tabs, from, 1),
    ArrowLeft: () => stepEnabledIndex(props.tabs, from, -1),
    Home: () => firstEnabledIndex(props.tabs),
    End: () => lastEnabledIndex(props.tabs),
  };
  const target = targets[event.key];
  if (!target) return;
  event.preventDefault();
  select(target());
}
</script>

<template>
  <div class="base-tabs">
    <div
      class="base-tabs__bar"
      :class="{ 'base-tabs__bar--solo': !$slots.bar, 'base-tabs__bar--list-end': listEnd }"
    >
      <div
        ref="listElement"
        class="base-tabs__list"
        :data-indicator="indicator ? (indicatorAnimated ? 'animated' : 'placed') : undefined"
        role="tablist"
        :aria-label="label"
        @keydown="onKeydown"
      >
        <button
          v-for="(tab, index) in tabs"
          :id="tabId(index)"
          :key="String(tab.value)"
          :ref="(element) => (tabElements[index] = element as HTMLElement)"
          class="base-tabs__tab"
          type="button"
          role="tab"
          :aria-selected="index === selectedIndex"
          :aria-controls="panelId"
          :tabindex="index === selectedIndex ? 0 : -1"
          :disabled="tab.disabled"
          :data-guide="tab.guide"
          @click="select(index)"
        >
          <component
            :is="tab.icon"
            v-if="tab.icon"
            class="base-tabs__icon"
          />
          {{ tab.label }}
          <span
            v-if="tab.badge !== undefined"
            class="base-tabs__badge"
          >{{ tab.badge }}</span>
        </button>
        <span
          v-if="indicator"
          class="base-tabs__indicator"
          aria-hidden="true"
          :style="{ transform: `translate(${indicator.x}px, ${indicator.y}px)`, width: `${indicator.width}px` }"
        />
      </div>
      <slot name="bar" />
    </div>
    <div
      :id="panelId"
      ref="panelElement"
      class="base-tabs__panel"
      role="tabpanel"
      tabindex="0"
      :aria-labelledby="selectedIndex >= 0 ? tabId(selectedIndex) : undefined"
    >
      <slot :value="model" />
    </div>
  </div>
</template>

<style scoped>
.base-tabs {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.base-tabs__bar {
  display: flex;
  flex: none;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: var(--space-2) var(--space-3);
  border-bottom: var(--border-width) solid var(--color-border);
}

.base-tabs__list {
  position: relative;
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
}

/* One accent line that slides under the selected tab; the per-tab line below is the fallback before it is measured. */
.base-tabs__indicator {
  position: absolute;
  top: 0;
  left: 0;
  height: 2px;
  background: var(--color-accent);
  pointer-events: none;
}

.base-tabs__list[data-indicator='animated'] > .base-tabs__indicator {
  transition:
    transform var(--duration-slow) var(--ease-out),
    width var(--duration-slow) var(--ease-out);
}

.base-tabs__list[data-indicator] > .base-tabs__tab[aria-selected='true']::after {
  display: none;
}

.base-tabs__bar--solo > .base-tabs__list {
  flex: 1;
}

.base-tabs__bar--list-end > .base-tabs__list {
  order: 2;
}

.base-tabs__tab {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: var(--space-3);
  height: var(--control-height-md);
  padding: 0 var(--space-5);
  border: var(--border-width) solid transparent;
  border-bottom: 0;
  border-radius: var(--radius-md) var(--radius-md) 0 0;
  background: transparent;
  color: var(--color-text-muted);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-regular);
  cursor: pointer;
  outline: none;
  transform: none;
  transition:
    background-color var(--duration-fast) var(--ease-out),
    color var(--duration-fast) var(--ease-out);
}

.base-tabs__tab:hover {
  background: var(--color-bg-hover);
  color: var(--color-text);
}

.base-tabs__tab[aria-selected='true'] {
  border-color: var(--color-border);
  background: var(--color-bg-panel-header);
  color: var(--color-text);
}

.base-tabs__tab[aria-selected='true']::after {
  position: absolute;
  right: 0;
  bottom: -1px;
  left: 0;
  height: 2px;
  background: var(--color-accent);
  content: '';
}

.base-tabs__tab:focus-visible {
  box-shadow: var(--focus-ring);
}

.base-tabs__badge {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.base-tabs__tab:disabled {
  background: transparent;
  color: var(--color-text-disabled);
  cursor: default;
}

.base-tabs__panel {
  flex: 1;
  min-height: 0;
  outline: none;
}
</style>
