<script setup lang="ts">
import { computed } from 'vue';
import DemoFrame from './DemoFrame.vue';
import { demoTourScenarios, type DemoTourId } from '../../../../src/web/demo/tours.ts';
import { tourRegistry } from '../../../../src/web/features/guided-help/model/registry.ts';

/**
 * Semantic tour embed for articles. The tour contract owns its scenario and
 * route, so Markdown never duplicates those implementation details.
 */
const props = defineProps<{
  id: DemoTourId;
  locale?: string;
  title: string;
  autoload?: boolean;
  minimal?: boolean;
  scale?: 0.25 | 0.5 | 0.75 | 1;
  viewportHeight?: number;
}>();

const tour = computed(() => tourRegistry[props.id]);
const scenario = computed(() => demoTourScenarios[props.id]);
</script>

<template>
  <DemoFrame
    :scenario="scenario"
    :route="tour.route"
    :tour="props.id"
    :locale="props.locale"
    :title="props.title"
    :autoload="props.autoload"
    :minimal="props.minimal"
    :scale="props.scale"
    :viewport-height="props.viewportHeight"
  />
</template>
