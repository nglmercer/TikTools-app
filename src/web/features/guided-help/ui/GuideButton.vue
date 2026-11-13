<script setup lang="ts">
import { computed, inject, nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { isLocale } from '@/shared/i18n/supported-locales.ts';
import { routerKey } from 'vue-router';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import { startInlineGuide, startTour } from '../model/coordinator.ts';
import { tourRegistry } from '../model/registry.ts';
import { tourCatalogs } from '../catalogs/index.ts';
import type { GuideSession, GuideStep, TourCapabilities, TourId, TourLocale, TourTaskState } from '../model/types.ts';
export type { GuideStep } from '../model/types.ts';
const props = withDefaults(defineProps<{
  id?: string;
  steps?: readonly GuideStep[];
  completed?: boolean;
  tour?: TourId;
  capabilities?: TourCapabilities;
  taskState?: TourTaskState | (() => TourTaskState);
}>(), { steps: () => [], id: undefined, tour: undefined, capabilities: undefined, taskState: undefined });
const { t, locale } = useI18n();
const router = inject(routerKey, null);
const guideLocale = computed<TourLocale>(() => isLocale(locale.value) ? locale.value : 'en');
const label = computed(() => {
  if (!props.tour) return t('guides.help');
  const tour = tourRegistry[props.tour];
  const copy = tourCatalogs[guideLocale.value][props.tour] as Record<string, { title: string }>;
  return `${t('guides.help')}: ${copy[tour.steps[0].id]!.title}`;
});
const active = ref(false);
let session: GuideSession | undefined;
function stop(): void { session?.stop('disposed'); session = undefined; active.value = false; }
function start(event: MouseEvent): void {
  if (session) { stop(); return; }
  const trigger = event.currentTarget as HTMLElement;
  const context = {
    trigger, scope: trigger.closest('[data-guide-scope]') ?? undefined,
  };
  const next = props.tour ? startTour(props.tour, {
    ...context, router: router ?? undefined, locale: guideLocale.value,
    capabilities: props.capabilities,
    taskState: () => ({
      ...(typeof props.taskState === 'function' ? props.taskState() : props.taskState),
      ...(props.completed === true ? { [tourRegistry[props.tour!].task]: true } : {}),
    }),
  }) : startInlineGuide({
    ...context, id: props.id ?? 'inline', steps: () => props.steps ?? [], completed: () => props.completed === true,
    onActiveChange: (value) => { active.value = value; },
    labels: {
      progress: t('guides.progress', { current: '{{current}}', total: '{{total}}' }),
      next: t('guides.next'), previous: t('guides.previous'), done: t('guides.done'), close: t('common.close'),
    },
  });
  session = next;
  void next.ready.then((value) => { if (session === next) active.value = value; });
  void next.done.then(() => { if (session === next) { session = undefined; active.value = false; } });
}
watch(() => [props.steps, props.completed, props.taskState, props.capabilities], async () => { await nextTick(); session?.refresh(); }, { deep: true });
watch(() => [props.id, props.tour], stop);
onBeforeUnmount(stop);
</script>
<template>
  <BaseIconButton
    :label="label"
    :data-guide-tour="tour"
    :pressed="active"
    size="sm"
    @click="start"
  >
    ?
  </BaseIconButton>
</template>
