<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { viewerGrant, type Grant, type Principal } from '@/entities/collaboration/api/types.ts';
import { areaById } from '@/features/manage-collaboration/model/access-areas.ts';
import { presetGrant } from '@/features/manage-collaboration/model/presets.ts';
import AccessInspector from '@/features/manage-collaboration/ui/AccessInspector.vue';
import AccessMatrix from '@/features/manage-collaboration/ui/AccessMatrix.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
const { t } = useI18n();
const disabled = ref(false);
const newGrant = ref<Grant>(viewerGrant());
const person: Principal = { principalId: 'sample-principal', clientLabel: 'Moderator PC', clientKind: 'desktop', grantRevision: '1', revoked: false, grant: presetGrant('workflow') };
const selected = ref<'workflows' | 'points'>('workflows');
</script>

<template>
  <section class="grant-kit">
    <h2>{{ t('collaboration.accessTitle') }}</h2>
    <p>{{ t('designKit.description') }}</p>
    <BaseSwitch v-model="disabled">
      {{ t('designKit.disabled') }}
    </BaseSwitch>
    <AccessMatrix
      :requests="[]"
      :principals="[person]"
      :online="{ 'sample-principal': 1 }"
      :drafts="{}"
      :new-grant="newGrant"
      :selection="{ target: 'new', area: selected }"
      :blocked="disabled"
      :can-invite="!disabled"
      @select="(_target, area) => { selected = area === 'points' ? 'points' : 'workflows' }"
      @preset="newGrant = presetGrant($event)"
    />
    <AccessInspector
      v-model="newGrant"
      :title="t('collaboration.newInvitation')"
      :area="areaById(selected)"
      reset-key="kit"
      :disabled="disabled"
    />
  </section>
</template>

<style scoped>
.grant-kit { display:grid; gap:var(--space-5); min-width:0; padding:var(--space-6); border:1px solid var(--color-border); background:var(--color-bg-panel); }
.grant-kit h2 { margin:0; font-size:var(--font-size-lg); }
.grant-kit p { margin:0; color:var(--color-text-muted); }
</style>
