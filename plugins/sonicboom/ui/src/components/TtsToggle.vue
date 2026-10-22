<script setup lang="ts">
import { ref, useId } from 'vue';
import TtsIcon from './TtsIcon.vue';
defineProps<{ label: string; hint?: string; checked: boolean; disabled?: boolean }>();
const emit = defineEmits<{ change: [checked: boolean] }>();
const id = useId();
const dismissed = ref(false);
</script>
<template>
  <label class="tts-toggle" :class="{ 'is-checked': checked, 'is-disabled': disabled, 'is-hint-dismissed': dismissed }" :title="hint || label" @keydown.esc="dismissed = true" @mouseleave="dismissed = false" @focusout="dismissed = false">
    <input type="checkbox" :checked="checked" :disabled="disabled" :aria-label="label" :aria-describedby="hint ? id : undefined" @change="emit('change', ($event.target as HTMLInputElement).checked)">
    <span class="tts-toggle__box"><TtsIcon name="check"/></span>
    <span class="tts-toggle__label">{{ label }}</span>
    <TtsIcon v-if="hint" name="info" class="tts-toggle__info"/>
    <span v-if="hint" :id="id" class="tts-toggle__hint" role="tooltip">{{ hint }}</span>
  </label>
</template>
