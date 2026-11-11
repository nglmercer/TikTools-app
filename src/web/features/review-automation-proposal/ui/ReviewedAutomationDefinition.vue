<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';
import type { ReviewedDefinition } from '../model/contracts.ts';

defineProps<{ title: string; definition: ReviewedDefinition; revision: string }>();
const { t } = useI18n();
</script>

<template>
  <section class="reviewed-definition">
    <h3>{{ title }}</h3>
    <p>{{ t('automationProposal.revision', { revision }) }}</p>
    <template v-if="definition.provenance">
      <p>{{ t(definition.provenance.authority.kind === 'owner' ? 'automationProposal.owner' : 'automationProposal.delegated') }}</p>
      <template v-if="definition.provenance.authority.kind === 'delegated'">
        <p>{{ t('automationProposal.principal', { id: definition.provenance.authority.principalId }) }}</p>
        <p>{{ t('automationProposal.session', { id: definition.provenance.authority.sessionId }) }}</p>
      </template>
      <p>{{ t('automationProposal.generation', { generation: definition.provenance.generation }) }}</p>
    </template>
    <p
      v-else
      role="alert"
    >
      {{ t('automationProposal.missingAuthority') }}
    </p>
    <BaseJsonView
      v-if="definition.record"
      :value="definition.record"
      max-height="34vh"
    />
    <p v-else>
      {{ t('automationProposal.absent') }}
    </p>
  </section>
</template>

<style scoped>
.reviewed-definition { min-width: 0; overflow-wrap: anywhere; }
.reviewed-definition h3 { margin: 0 0 var(--space-2); font-size: var(--font-size-md); }
.reviewed-definition p { margin: var(--space-2) 0; }
</style>
