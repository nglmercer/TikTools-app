<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseStatusAction from '@/shared/ui/status/BaseStatusAction.vue';
import type { DiagnosticSource } from './log.ts';
import { useDiagnosticNavigation } from './navigation.ts';
withDefaults(defineProps<{ label: string; source: DiagnosticSource; entityId?: string; tone?: 'error' | 'warning' | 'info' }>(), { entityId: '', tone: 'error' });
const { t } = useI18n();
const navigation = useDiagnosticNavigation();
</script>
<template>
  <BaseStatusAction
    class="diagnostic-action"
    :class="`diagnostic-action--${tone}`"
    :label="label"
    :action-label="t('diagnostics.viewLogs')"
    :tone="tone"
    @click.stop="navigation.open(source, entityId)"
  />
</template>
