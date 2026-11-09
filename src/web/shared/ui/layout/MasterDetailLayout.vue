<script setup lang="ts">
import { nextTick, ref, watch } from 'vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseSplitter from '@/shared/ui/splitter/BaseSplitter.vue';

const props = withDefaults(defineProps<{
  narrow: boolean;
  detailOpen: boolean;
  backLabel: string;
  detailTitle?: string;
  min?: number;
  max?: number;
}>(), { detailTitle: '', min: 240, max: 900 });
const size = defineModel<number>('size', { default: 300 });
defineEmits<{ back: [] }>();
const root = ref<HTMLElement | null>(null);
let returnFocus: HTMLElement | null = null;
watch(() => props.narrow && props.detailOpen, async (open, previous) => {
  if (open) {
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    await nextTick();
    root.value?.querySelector<HTMLButtonElement>('.master-detail__back button')?.focus();
  } else if (previous && returnFocus?.isConnected) {
    await nextTick();
    returnFocus.focus();
  }
});
</script>

<template>
  <div
    ref="root"
    class="master-detail"
    :class="{ 'master-detail--narrow': narrow, 'master-detail--detail': narrow && detailOpen }"
  >
    <BaseSplitter
      v-model:size="size"
      :min="min"
      :max="max"
      class="master-detail__splitter"
    >
      <template #start>
        <!-- Keep the collection and editor mounted so resizing never discards a draft. -->
        <div
          class="master-detail__master"
          :hidden="narrow && detailOpen"
        >
          <slot name="master" />
        </div>
      </template>
      <template #end>
        <div
          class="master-detail__detail"
          :hidden="narrow && !detailOpen"
        >
          <header
            v-if="narrow && detailOpen"
            class="master-detail__header"
          >
            <span class="master-detail__back"><BaseButton @click="$emit('back')"><template #icon><IconChevronLeft /></template>{{ backLabel }}</BaseButton></span>
            <span
              v-if="detailTitle"
              class="master-detail__title"
            >{{ detailTitle }}</span>
            <slot name="detail-actions" />
          </header>
          <div class="master-detail__body">
            <slot name="detail" />
          </div>
        </div>
      </template>
    </BaseSplitter>
  </div>
</template>

<style scoped>
.master-detail, .master-detail__splitter, .master-detail__master, .master-detail__detail, .master-detail__body { display:flex; flex:1; min-width:0; min-height:0; }
.master-detail__master, .master-detail__detail, .master-detail__body { flex-direction:column; }
.master-detail__master[hidden], .master-detail__detail[hidden] { display:none; }
.master-detail__header { display:flex; align-items:center; flex-wrap:wrap; flex:none; gap:var(--space-3); padding:var(--space-3); border-bottom:var(--border-width) solid var(--color-border-subtle); background:var(--color-bg-panel-header); }
.master-detail__title { flex:1 1 140px; min-width:0; overflow-wrap:anywhere; font-weight:var(--font-weight-semibold); }
.master-detail__body { overflow:auto; }
.master-detail--narrow > :deep(.base-splitter) { flex-direction:column; }
.master-detail--narrow > :deep(.base-splitter > .base-splitter__handle) { display:none; }
.master-detail--narrow > :deep(.base-splitter > .base-splitter__pane) { flex:1 1 auto !important; flex-basis:auto !important; }
.master-detail--narrow:not(.master-detail--detail) > :deep(.base-splitter > .base-splitter__pane:last-child),
.master-detail--detail > :deep(.base-splitter > .base-splitter__pane:first-child) { display:none; }
.master-detail--narrow .master-detail__back :deep(button) { min-height:44px; }
</style>
