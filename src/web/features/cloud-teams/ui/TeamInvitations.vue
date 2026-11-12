<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { ref } from 'vue';
import { useTeamsStore } from '@/entities/cloud-teams/model/teams-store.ts';
import type { TeamRole } from '@/entities/cloud-teams/api/teams-api.ts';
import { useTeamLabels } from '../lib/use-team-labels.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
const { t } = useI18n();
const teams = useTeamsStore();
const inviteRole = ref<TeamRole>('observer');
const inviteTtl = ref<number>(900);
const copied = ref(false);
const { roleOptions, ttlOptions, statusLabel } = useTeamLabels();
async function copyToken(): Promise<void> {
  const token = teams.issued?.token;
  if (!token) return;
  try {
    await navigator.clipboard.writeText(token);
    copied.value = true;
  } catch {
    copied.value = false;
  }
}


</script>

<template>
  <div class="cloud-teams__block">
    <h4>{{ t('cloudTeams.invitations') }}</h4>
    <div class="cloud-teams__row">
      <BaseComboBox
        v-model="inviteRole"
        :options="roleOptions"
        :aria-label="t('cloudTeams.role')"
      />
      <BaseComboBox
        v-model="inviteTtl"
        :options="ttlOptions"
        :aria-label="t('cloudTeams.validFor')"
      />
      <BaseButton
        :disabled="teams.working"
        @click="teams.createInvitation(inviteRole, inviteTtl)"
      >
        {{ t('cloudTeams.createInvitation') }}
      </BaseButton>
    </div>
    <p class="cloud-teams__note">
      {{ t('cloudTeams.inviteNote') }}
    </p>
    <BaseAlert
      v-if="teams.issued"
      tone="warning"
      :title="t('cloudTeams.tokenOnce')"
    >
      <code class="cloud-teams__code">{{ teams.issued.token }}</code>
      <div class="cloud-teams__row">
        <BaseButton
          size="sm"
          @click="copyToken"
        >
          {{ copied ? t('cloudTeams.copied') : t('cloudTeams.copyToken') }}
        </BaseButton>
        <BaseButton
          size="sm"
          variant="ghost"
          @click="teams.dismissInvitation()"
        >
          {{ t('cloudTeams.done') }}
        </BaseButton>
      </div>
    </BaseAlert>
    <ul class="cloud-teams__list">
      <li
        v-for="invitation in teams.invitations"
        :key="invitation.id"
        class="cloud-teams__row"
      >
        <span class="cloud-teams__grow">
          {{ t(`cloudTeams.roles.${invitation.requestedRole}`) }} ·
          {{ statusLabel('cloudTeams.inviteStatus', invitation.status) }}
        </span>
        <BaseButton
          v-if="invitation.status === 'open'"
          size="sm"
          variant="ghost"
          @click="teams.revokeInvitation(invitation.id)"
        >
          {{ t('cloudTeams.revoke') }}
        </BaseButton>
      </li>
    </ul>
  </div>
</template>
