<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { DiagnosticSource } from './log.ts';
import { useDiagnosticNavigation } from './navigation.ts';
withDefaults(defineProps<{ label: string; source: DiagnosticSource; entityId?: string; tone?: 'error' | 'warning' | 'info' }>(), { entityId: '', tone: 'error' });
const { t } = useI18n();
const navigation = useDiagnosticNavigation();
</script>

<template>
  <button
    type="button"
    class="diagnostic-action"
    :class="`diagnostic-action--${tone}`"
    :aria-label="`${label}. ${t('diagnostics.viewLogs')}`"
    @click.stop="navigation.open(source, entityId)"
  >
    <span aria-hidden="true">●</span>
    <span>{{ label }}</span>
    <span
      class="diagnostic-action__hint"
      aria-hidden="true"
    >{{ t('diagnostics.viewLogs') }} ↗</span>
  </button>
</template>

<style scoped>
.diagnostic-action { display: inline-flex; align-items: center; gap: 6px; max-width: 100%; padding: 4px 2px; border: 0; background: transparent; color: var(--color-text); font: inherit; text-align: left; cursor: pointer; }
.diagnostic-action > span:first-child { color: var(--color-danger-hover); font-size: 10px; }
.diagnostic-action--warning > span:first-child { color: var(--color-warning); }
.diagnostic-action--info > span:first-child { color: var(--color-text-muted); }
.diagnostic-action__hint { color: var(--color-text-muted); text-decoration: underline; white-space: nowrap; }
.diagnostic-action:focus-visible { outline: 2px solid var(--color-border-focus); outline-offset: 2px; }
@media (hover: hover) and (pointer: fine) and (min-width: 768px) {
  .diagnostic-action__hint { opacity: 0; }
  .diagnostic-action:hover .diagnostic-action__hint, .diagnostic-action:focus-visible .diagnostic-action__hint { opacity: 1; }
}
@media (any-pointer: coarse) { .diagnostic-action { min-height: 44px; } .diagnostic-action__hint { opacity: 1; } }
</style>
