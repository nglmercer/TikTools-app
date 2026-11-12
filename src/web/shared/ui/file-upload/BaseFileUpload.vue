<script setup lang="ts">
import { ref, useTemplateRef, watch, onBeforeUnmount } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';

const props = withDefaults(defineProps<{
  label: string;
  hint: string;
  accept: string;
  maxBytes: number;
  disabled?: boolean;
}>(), { disabled: false });
const file = defineModel<File | null>({ default: null });
const emit = defineEmits<{ error: [message: string] }>();
const { t } = useI18n();
const input = useTemplateRef('input');
const dragging = ref(false);
const error = ref('');
const preview = ref('');
function clearPreview(): void { if (preview.value) URL.revokeObjectURL(preview.value); preview.value = ''; }
watch(file, value => { clearPreview(); if (value?.type.startsWith('image/')) preview.value = URL.createObjectURL(value); }, { immediate: true });
onBeforeUnmount(clearPreview);
let depth = 0;

function fail(message: string): void {
  error.value = message;
  file.value = null;
  emit('error', message);
}
function choose(files: FileList | File[]): void {
  if (props.disabled || !files.length) return;
  error.value = '';
  if (files.length !== 1) { fail(t('fileUpload.single')); return; }
  const selected = files[0]!;
  if (!selected.size || selected.size > props.maxBytes) {
    fail(t('fileUpload.tooLarge', { size: Math.ceil(props.maxBytes / 1024) })); return;
  }
  const allowed = props.accept.split(',').map(value => value.trim().toLowerCase());
  const valid = allowed.some(value => value.startsWith('.')
    ? selected.name.toLowerCase().endsWith(value)
    : value.endsWith('/*') ? selected.type.toLowerCase().startsWith(value.slice(0, -1)) : selected.type.toLowerCase() === value);
  if (!valid) { fail(t('fileUpload.invalid')); return; }
  file.value = selected;
}
function picked(event: Event): void {
  const target = event.target as HTMLInputElement;
  if (target.files) choose(target.files);
  target.value = '';
}
function dropped(event: DragEvent): void {
  depth = 0; dragging.value = false;
  if (event.dataTransfer?.files) choose(event.dataTransfer.files);
}
function enter(): void { if (!props.disabled) { depth += 1; dragging.value = true; } }
function leave(): void { depth = Math.max(0, depth - 1); if (!depth) dragging.value = false; }
function browse(): void { if (!props.disabled) input.value?.click(); }
</script>

<template>
  <div class="base-file-upload">
    <div
      class="base-file-upload__zone"
      :class="{ 'base-file-upload__zone--dragging': dragging && !disabled }"
      role="group"
      :aria-label="label"
      :aria-disabled="disabled"
      @dragenter.prevent="enter"
      @dragover.prevent
      @dragleave.prevent="leave"
      @drop.prevent="dropped"
    >
      <span
        class="base-file-upload__icon"
        aria-hidden="true"
      ><IconUpload :size="24" /></span>
      <strong>{{ label }}</strong>
      <span class="base-file-upload__hint">{{ hint }}</span>
      <BaseButton
        :disabled="disabled"
        @click="browse"
      >
        {{ t('fileUpload.browse') }}
      </BaseButton>
      <input
        ref="input"
        class="base-file-upload__input"
        type="file"
        :accept="accept"
        :disabled="disabled"
        :aria-label="label"
        tabindex="-1"
        @change="picked"
      >
    </div>
    <div
      v-if="file"
      class="base-file-upload__selection"
      role="status"
    >
      <img
        v-if="preview"
        :src="preview"
        alt=""
        class="base-file-upload__thumbnail"
      >
      <div><strong>{{ file.name }}</strong><span>{{ Math.max(1, Math.ceil(file.size / 1024)) }} KB</span></div>
      <BaseButton
        variant="ghost"
        :disabled="disabled"
        @click="file = null; error = ''"
      >
        {{ t('fileUpload.remove') }}
      </BaseButton>
    </div>
    <p
      v-if="error"
      class="base-file-upload__error"
      role="alert"
    >
      {{ error }}
    </p>
  </div>
</template>

<style scoped>
.base-file-upload { display:grid; gap:var(--space-5); min-width:0; }
.base-file-upload__zone { display:flex; flex-direction:column; align-items:center; justify-content:center; gap:var(--space-5); min-height:168px; padding:var(--space-7); border:1px dashed var(--color-border-strong); border-radius:var(--radius-md); background:var(--color-bg-field); text-align:center; }
.base-file-upload__zone:focus-within { outline:1px solid var(--color-border-focus); outline-offset:2px; }
.base-file-upload__zone--dragging { border-color:var(--color-border-selected); background:var(--color-bg-selected); }
.base-file-upload__zone[aria-disabled="true"] { opacity:0.6; }
.base-file-upload__icon { display:grid; place-items:center; padding:var(--space-5); background:var(--color-bg-panel); border:1px solid var(--color-border); border-radius:var(--radius-md); }
.base-file-upload__hint, .base-file-upload__selection span { color:var(--color-text-muted); font-size:var(--font-size-sm); }
.base-file-upload__input { display:none; }
.base-file-upload__selection { display:flex; align-items:center; gap:var(--space-5); padding:var(--space-5); border:1px solid var(--color-border); background:var(--color-bg-panel); }
.base-file-upload__selection > div { display:grid; flex:1; min-width:0; gap:var(--space-2); overflow-wrap:anywhere; }
.base-file-upload__error { margin:0; color:var(--color-text-danger); }
.base-file-upload__thumbnail { width:48px; height:48px; flex:none; object-fit:contain; background:var(--color-bg-app); border:1px solid var(--color-border); border-radius:var(--radius-md); }
@media (width < 700px) {
 .base-file-upload__zone { min-height:120px; padding:var(--space-6); gap:var(--space-4); }
}
</style>
