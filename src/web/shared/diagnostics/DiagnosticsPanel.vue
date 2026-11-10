<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseFloatingPanel from '@/shared/ui/floating-panel/BaseFloatingPanel.vue';
import DiagnosticsLog from './DiagnosticsLog.vue';
import { diagnosticView, type DiagnosticSource } from './log.ts';
import { useDiagnosticNavigation } from './navigation.ts';
const { t } = useI18n();
const navigation = useDiagnosticNavigation();
const { open, request } = diagnosticView;
function onViewportChange(width: number): void {
  if (open.value && width < 768) navigation.open((diagnosticView.source.value || undefined) as DiagnosticSource | undefined, diagnosticView.entityId.value);
}
</script>
<template>
  <BaseFloatingPanel
    :open="open"
    class="diagnostic-panel"
    :activation-key="request"
    :title="t('diagnostics.title')"
    :move-label="t('diagnostics.move')"
    :close-label="t('diagnostics.close')"
    :minimize-label="t('diagnostics.minimize')"
    :restore-label="t('diagnostics.restore')"
    :reset-label="t('diagnostics.reset')"
    @update:open="!$event && navigation.close()"
    @viewport-change="onViewportChange"
  >
    <template #default>
      <DiagnosticsLog v-if="open" />
    </template>
  </BaseFloatingPanel>
</template>

<style scoped>
.diagnostic-panel { box-shadow: none; border-radius: 0; }
</style>
