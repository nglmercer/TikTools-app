<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Driver } from 'driver.js';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
export interface GuideStep { id: string; selector: string; title: string; description: string }
const props = defineProps<{ id: string; steps: readonly GuideStep[]; completed?: boolean }>();
const { t } = useI18n();
const active = ref(false);
let tour: Driver | undefined;
let trigger: HTMLElement | undefined;
let scope: Element | Document = document;
let generation = 0;
let observer: MutationObserver | undefined;
const storageKey = () => `tiktools.guide.${props.id}`;
function escape(value: string): string { return value.replace(/[&<>"']/g, character => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character]!); }
function stop(): void { generation++; observer?.disconnect(); observer = undefined; tour?.destroy(); tour = undefined; active.value = false; }
function show(): void {
  if (!tour || !active.value) return;
  const visible = props.steps.map(step => ({ ...step, element: scope.querySelector(step.selector) })).filter(step => step.element instanceof HTMLElement && step.element.isConnected && !step.element.closest('[hidden]') && !step.element.matches(':disabled'));
  if (!visible.length || props.completed) { stop(); return; }
  let saved: string | null = null;
  try { saved = sessionStorage.getItem(storageKey()); } catch { /* Storage is optional. */ }
  tour.setSteps(visible.map(step => ({ element: step.element!, data: { id: step.id }, popover: { title: escape(step.title), description: escape(step.description) } })));
  tour.setConfig({ ...tour.getConfig(), showProgress: visible.length > 1, showButtons: visible.length > 1 ? ['next', 'previous', 'close'] : ['close'] });
  tour.drive(Math.max(0, visible.findIndex(step => step.id === saved)));
}
async function start(event: MouseEvent): Promise<void> {
  if (active.value) { stop(); return; }
  trigger = event.currentTarget as HTMLElement;
  scope = trigger.closest('[data-guide-scope]') ?? document;
  const ticket = ++generation;
  const [{ driver }] = await Promise.all([import('driver.js'), import('driver.js/dist/driver.css')]);
  if (ticket !== generation) return;
  active.value = true;
  tour = driver({
    animate: !window.matchMedia('(prefers-reduced-motion: reduce)').matches,
    allowClose: true, allowKeyboardControl: true, popoverClass: 'tiktools-guide',
    allowedElements: scope instanceof Element ? [scope] : [],
    progressText: t('guides.progress', { current: '{{current}}', total: '{{total}}' }),
    nextBtnText: t('guides.next'), prevBtnText: t('guides.previous'), doneBtnText: t('guides.done'), closeBtnLabel: t('common.close'),
    onHighlighted: (_element, step) => { try { sessionStorage.setItem(storageKey(), String(step.data?.id ?? '')); } catch { /* Storage is optional. */ } },
    onDestroyed: () => { active.value = false; observer?.disconnect(); if (trigger?.isConnected) trigger.focus({ preventScroll: true }); },
  });
  observer = new MutationObserver(() => { if (document.querySelector('dialog[open]')) stop(); });
  observer.observe(document.body, { attributes: true, subtree: true, attributeFilter: ['open'] });
  await nextTick();
  if (ticket === generation) show();
}
watch(() => [props.steps, props.completed], async () => { await nextTick(); show(); }, { deep: true });
onBeforeUnmount(stop);
</script>
<template>
  <BaseIconButton
    :label="t('guides.help')"
    :pressed="active"
    size="sm"
    @click="start"
  >
    ?
  </BaseIconButton>
</template>
<style>
.tiktools-guide.driver-popover { background: var(--color-bg-panel); color: var(--color-text); font-family: inherit; }
.tiktools-guide .driver-popover-title, .tiktools-guide .driver-popover-description { color: inherit; }
.tiktools-guide .driver-popover-progress-text { color: var(--color-text-muted); }
.tiktools-guide .driver-popover-footer button { background: var(--color-bg-panel-header); color: var(--color-text); border: 1px solid var(--color-border); text-shadow: none; }

.tiktools-guide .driver-popover-arrow-side-top { border-top-color: var(--color-bg-panel); }
.tiktools-guide .driver-popover-arrow-side-bottom { border-bottom-color: var(--color-bg-panel); }
.tiktools-guide .driver-popover-arrow-side-left { border-left-color: var(--color-bg-panel); }
.tiktools-guide .driver-popover-arrow-side-right { border-right-color: var(--color-bg-panel); }
</style>
