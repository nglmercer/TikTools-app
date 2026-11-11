<script setup lang="ts">
import { onMounted, useId, useTemplateRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconClose from '@/shared/ui/icon/icons/IconClose.vue';

withDefaults(
  defineProps<{
    title: string;
    width?: number;
    closable?: boolean;
  }>(),
  { width: 480, closable: true },
);

const open = defineModel<boolean>('open', { default: false });
const { t } = useI18n();
const titleId = useId();
const dialog = useTemplateRef('dialog');

function sync(isOpen: boolean): void {
  const element = dialog.value;
  if (!element) return;
  if (isOpen && !element.open) element.showModal();
  else if (!isOpen && element.open) element.close();
}

function onCancel(event: Event, closable: boolean): void {
  if (!closable) event.preventDefault();
}

watch(open, sync, { flush: 'post' });
onMounted(() => sync(open.value));
</script>

<template>
  <dialog
    ref="dialog"
    class="base-dialog"
    :aria-labelledby="titleId"
    :style="{ width: `${width}px` }"
    @cancel="onCancel($event, closable)"
    @close="open = false"
  >
    <template v-if="open">
      <header class="base-dialog__header">
        <h2
          :id="titleId"
          class="base-dialog__title"
        >
          {{ title }}
        </h2>
        <slot name="header-actions" />
        <BaseIconButton
          v-if="closable"
          :label="t('common.close')"
          size="sm"
          @click="open = false"
        >
          <IconClose />
        </BaseIconButton>
      </header>
      <div class="base-dialog__body">
        <slot />
      </div>
      <footer
        v-if="$slots.footer || $slots['footer-start']"
        class="base-dialog__footer"
      >
        <slot name="footer-start" />
        <span class="base-dialog__spacer" />
        <slot name="footer" />
      </footer>
    </template>
  </dialog>
</template>

<style scoped>
.base-dialog {
  max-width: calc(100vw - var(--space-7) * 2);
  max-height: calc(100vh - var(--space-7) * 2);
  padding: 0;
  border: var(--border-width) solid var(--color-border-strong);
  border-radius: var(--radius-md);
  background: var(--color-bg-panel);
  box-shadow: var(--shadow-popup);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
}

.base-dialog[open] {
  display: flex;
  flex-direction: column;
}

.base-dialog::backdrop {
  background: rgb(0 0 0 / 45%);
}

.base-dialog__header {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-3) var(--space-3) var(--space-5);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel-header);
}

.base-dialog__title {
  flex: 1;
  margin: 0;
  overflow: hidden;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-semibold);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.base-dialog__body {
  flex: 1;
  min-height: 0;
  padding: var(--space-5);
  overflow: auto;
}

.base-dialog__footer {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-4) var(--space-5);
}

.base-dialog__spacer {
  flex: 1;
}
</style>
