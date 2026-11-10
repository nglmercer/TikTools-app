<script setup lang="ts">
import { watch } from 'vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import DiagnosticsLog from '@/shared/diagnostics/DiagnosticsLog.vue';
import { diagnosticView } from '@/shared/diagnostics/log.ts';
const route = useRoute(); const router = useRouter(); const { t } = useI18n();
watch(() => route.query, query => { diagnosticView.open.value = false; diagnosticView.source.value = String(query.source ?? ''); diagnosticView.entityId.value = String(query.entity ?? ''); diagnosticView.selectedId.value = String(query.entry ?? ''); }, { immediate:true });
function back(): void { const target = String(route.query.returnTo ?? '/connections'); void router.push(target.startsWith('/') && !target.startsWith('//') && !target.startsWith('/debug') ? target : '/connections'); }
</script>
<template>
  <section class="debug-page">
    <header class="debug-page__header">
      <BaseIconButton
        :label="t('diagnostics.back')"
        @click="back"
      >
        <IconChevronLeft />
      </BaseIconButton>
      <h1>{{ t('diagnostics.title') }}</h1>
    </header>
    <DiagnosticsLog />
  </section>
</template>

<style scoped>
.debug-page { display: flex; flex-direction: column; flex: 1; min-width: 0; min-height: 0; }
.debug-page__header { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3) var(--space-4); border-bottom: var(--border-width) solid var(--color-border); background: var(--color-bg-panel-header); }
.debug-page__header h1 { margin: 0; font-size: var(--font-size-lg); }
.debug-page > :deep(.logs) { flex: 1; }
</style>
