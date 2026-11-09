<script setup lang="ts">
import { watch } from 'vue';
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
    <button @click="back">
      ← {{ t('diagnostics.back') }}
    </button><h1>{{ t('diagnostics.title') }}</h1><DiagnosticsLog />
  </section>
</template>

<style scoped>
.debug-page > button { margin: 8px 16px; min-height: 44px; padding: 8px 12px; color: var(--color-text); background: var(--color-bg-secondary, #20242d); border: 1px solid var(--color-border); border-radius: 6px; font: inherit; cursor: pointer; }
.debug-page > h1 { margin: 8px 16px; font-size: 22px; }
.debug-page > button:focus-visible { outline: 2px solid #a69bff; }
</style>
