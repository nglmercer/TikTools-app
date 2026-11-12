<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';

const props = withDefaults(
  defineProps<{
    title: string;
    hint: string;
    browseLabel: string;
    pasteLabel: string;
    pasteBlockedLabel: string;
    clipboardLabel: string;
    unreadableLabel: string;
    accept?: string;
    disabled?: boolean;
    /** Largest accepted file or pasted text, in bytes. Larger input is refused with `tooLargeLabel`. */
    maxBytes?: number;
    tooLargeLabel?: string;
  }>(),
  { accept: '.json,application/json', disabled: false, maxBytes: undefined, tooLargeLabel: '' },
);

const emit = defineEmits<{ content: [text: string, source: string]; error: [message: string] }>();

const zone = useTemplateRef('zone');
const input = useTemplateRef('input');
const dragging = ref(false);
const notice = ref('');

function tooLarge(bytes: number): boolean {
  if (props.maxBytes === undefined || bytes <= props.maxBytes) return false;
  emit('error', props.tooLargeLabel || props.unreadableLabel);
  return true;
}

function read(file: File): void {
  if (tooLarge(file.size)) return;
  const reader = new FileReader();
  reader.onload = () => emit('content', typeof reader.result === 'string' ? reader.result : '', file.name);
  reader.onerror = () => emit('error', props.unreadableLabel);
  reader.readAsText(file);
}

function browse(): void {
  if (!props.disabled) input.value?.click();
}

function onPicked(event: Event): void {
  const target = event.target as HTMLInputElement;
  const file = target.files?.[0];
  if (file) read(file);
  target.value = '';
}

function onDrop(event: DragEvent): void {
  dragging.value = false;
  const file = event.dataTransfer?.files?.[0];
  if (file && !props.disabled) read(file);
}

function onDocumentPaste(event: ClipboardEvent): void {
  const text = event.clipboardData?.getData('text');
  if (!text || props.disabled) return;
  event.preventDefault();
  notice.value = '';
  if (!tooLarge(new TextEncoder().encode(text).length)) emit('content', text, props.clipboardLabel);
}

async function paste(): Promise<void> {
  try {
    if (!navigator.clipboard?.readText) throw new Error('clipboard unavailable');
    const text = await navigator.clipboard.readText();
    notice.value = '';
    if (!tooLarge(new TextEncoder().encode(text).length)) emit('content', text, props.clipboardLabel);
  } catch {
    notice.value = props.pasteBlockedLabel;
    zone.value?.focus();
  }
}

onMounted(() => document.addEventListener('paste', onDocumentPaste));
onBeforeUnmount(() => document.removeEventListener('paste', onDocumentPaste));
</script>

<template>
  <div
    ref="zone"
    class="base-file-drop"
    :class="{ 'base-file-drop--dragging': dragging, 'base-file-drop--disabled': disabled }"
    role="group"
    :aria-label="title"
    tabindex="0"
    @dragover.prevent="dragging = true"
    @dragleave="dragging = false"
    @drop.prevent="onDrop"
    @keydown.enter.prevent="browse"
  >
    <p class="base-file-drop__title">
      {{ title }}
    </p>
    <p class="base-file-drop__hint">
      {{ hint }}
    </p>
    <div class="base-file-drop__actions">
      <BaseButton
        size="sm"
        :disabled="disabled"
        @click="browse"
      >
        {{ browseLabel }}
      </BaseButton>
      <BaseButton
        size="sm"
        :disabled="disabled"
        @click="paste"
      >
        {{ pasteLabel }}
      </BaseButton>
    </div>
    <p
      v-if="notice"
      class="base-file-drop__notice"
      role="status"
    >
      {{ notice }}
    </p>
    <input
      ref="input"
      class="base-file-drop__input"
      type="file"
      :accept="accept"
      tabindex="-1"
      aria-hidden="true"
      @change="onPicked"
    >
  </div>
</template>

<style scoped>
.base-file-drop {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-6) var(--space-5);
  border: var(--border-width) dashed var(--color-border-strong);
  border-radius: var(--radius-sm);
  background: var(--color-bg-field);
  color: var(--color-text);
  text-align: center;
  outline: none;
}

.base-file-drop:focus-visible {
  box-shadow: var(--focus-ring);
}

.base-file-drop--dragging {
  border-color: var(--color-accent);
  background: var(--color-bg-selected);
}

.base-file-drop--disabled {
  opacity: 0.6;
}

.base-file-drop__title {
  margin: 0;
  font-weight: var(--font-weight-semibold);
}

.base-file-drop__hint,
.base-file-drop__notice {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.base-file-drop__notice {
  color: var(--color-warning);
}

.base-file-drop__actions {
  display: flex;
  gap: var(--space-2);
}

.base-file-drop__input {
  display: none;
}
</style>
