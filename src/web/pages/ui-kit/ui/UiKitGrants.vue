<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { viewerGrant } from '@/entities/collaboration/api/types.ts';
import GrantEditor from '@/features/manage-collaboration/ui/GrantEditor.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
const { t } = useI18n();
const grant = ref(viewerGrant());
const disabled = ref(false);
</script>

<template>
  <section class="grant-kit">
    <h2>{{ t('collaboration.grantSummary') }}</h2>
    <p>{{ t('designKit.description') }}</p>
    <BaseSwitch v-model="disabled">
      {{ t('designKit.disabled') }}
    </BaseSwitch>
    <GrantEditor
      v-model="grant"
      :disabled="disabled"
    />
  </section>
</template>

<style scoped>
.grant-kit { display:grid; gap:var(--space-5); min-width:0; padding:var(--space-6); border:1px solid var(--color-border); background:var(--color-bg-panel); }
.grant-kit h2 { margin:0; font-size:var(--font-size-lg); }
.grant-kit p { margin:0; color:var(--color-text-muted); }
</style>
