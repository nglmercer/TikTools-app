<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseIconButton from './BaseIconButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';

/** The "back" icon button that leads a detail panel's header; listen to `back`. */
const emit = defineEmits<{ back: [] }>();
const { t } = useI18n();
</script>

<template>
  <BaseIconButton
    :label="t('common.back')"
    size="sm"
    @click="emit('back')"
  >
    <IconChevronLeft />
  </BaseIconButton>
</template>
