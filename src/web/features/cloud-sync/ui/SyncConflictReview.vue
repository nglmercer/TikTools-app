<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
defineProps<{ conflicts: readonly string[]; working: boolean }>();
const emit = defineEmits<{ resolve: [key: string, keepLocal: boolean] }>();
const { t } = useI18n();
</script>
<template>
  <section
    v-if="conflicts.length"
    class="sync-conflicts"
    :aria-label="t('formsDesign.conflictReview')"
  >
    <BaseAlert
      tone="warning"
      :title="t('sync.conflicts', { count: conflicts.length })"
    />
    <p class="sync-conflicts__hint">
      {{ t('formsDesign.conflictHint') }}
    </p>
    <ul>
      <li
        v-for="key in conflicts"
        :key="key"
      >
        <code>{{ key }}</code>
        <div
          class="sync-conflicts__actions"
          role="group"
          :aria-label="key"
        >
          <BaseButton
            :disabled="working"
            @click="emit('resolve', key, true)"
          >
            {{ t('sync.keepMine') }}
          </BaseButton>
          <BaseButton
            :disabled="working"
            @click="emit('resolve', key, false)"
          >
            {{ t('sync.useCloud') }}
          </BaseButton>
        </div>
      </li>
    </ul>
  </section>
</template>
<style scoped>
.sync-conflicts { border-top: var(--border-width) solid var(--color-border); padding: var(--space-5) 0; }
.sync-conflicts__hint { color: var(--color-text-muted); font-size: var(--font-size-sm); line-height: var(--line-height-normal); }
ul { list-style: none; margin: 0; padding: 0; }
li { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: var(--space-4); padding: var(--space-5) 0; border-bottom: var(--border-width) solid var(--color-border-subtle); }
code { font-family: var(--font-family-mono); font-size: var(--font-size-sm); overflow-wrap: anywhere; min-width: 0; }
.sync-conflicts__actions { display: flex; flex-wrap: wrap; gap: var(--space-3); }
@media (any-pointer: coarse) { .sync-conflicts__actions :deep(button) { min-height: 44px; } }
</style>
