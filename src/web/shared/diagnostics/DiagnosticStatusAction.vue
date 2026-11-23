<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseStatusAction from '@/shared/ui/status/BaseStatusAction.vue';
import { diagnosticLog, type DiagnosticSource } from './log.ts';
import { useDiagnosticNavigation } from './navigation.ts';
const props = withDefaults(defineProps<{ label: string; source: DiagnosticSource; entityId?: string; tone?: 'error' | 'warning' | 'info' }>(), { entityId: '', tone: 'error' });
const { t } = useI18n();
const navigation = useDiagnosticNavigation();
const detail = computed(() => diagnosticLog.entries.value.findLast((entry) =>
  entry.source === props.source && entry.entityId === props.entityId && entry.level === 'error',
)?.message ?? '');
</script>
<template>
  <BaseStatusAction
    class="diagnostic-action"
    :class="`diagnostic-action--${tone}`"
    :label="label"
    :action-label="t('diagnostics.viewLogs')"
    :tone="tone"
    :detail="detail"
    @click.stop="navigation.open(source, entityId)"
  />
</template>
