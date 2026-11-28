<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseNotification from '@/shared/ui/notification/BaseNotification.vue';
import { pausedGuide } from '../model/paused-guide.ts';

const { t } = useI18n();
/** Starting the guide again clears the pause, so the notice leaves on its own. */
function resume(): void {
  const pause = pausedGuide.value;
  pausedGuide.value = undefined;
  pause?.resume();
}
</script>

<template>
  <BaseNotification
    v-if="pausedGuide"
    class="app-layout__notice"
    :message="t('guides.paused', { step: pausedGuide.title })"
    :action-label="t('guides.resume')"
    @action="resume()"
    @dismiss="pausedGuide = undefined"
  />
</template>
