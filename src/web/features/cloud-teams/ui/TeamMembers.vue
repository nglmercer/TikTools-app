<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { useTeamsStore } from '@/entities/cloud-teams/model/teams-store.ts';
import type { TeamGrant, TeamRole } from '@/entities/cloud-teams/api/teams-api.ts';
import { useTeamLabels } from '../lib/use-team-labels.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
const { t } = useI18n();
const teams = useTeamsStore();
const { roleOptions, statusLabel } = useTeamLabels();
function changeRole(grant: TeamGrant, role: TeamRole): void {
  if (grant.role !== role) void teams.changeRole(grant, role);
}


</script>

<template>
  <div class="cloud-teams__membership">
    <div class="cloud-teams__block">
      <h4>{{ t('cloudTeams.requests') }}</h4>
      <ul class="cloud-teams__list">
        <li
          v-for="request in teams.requests"
          :key="request.id"
          class="cloud-teams__row"
        >
          <span class="cloud-teams__grow">
            {{ request.deviceLabel }} · {{ t(`cloudTeams.roles.${request.requestedRole}`) }} ·
            {{ statusLabel('cloudTeams.requestStatus', request.status) }}
          </span>
          <template v-if="request.status === 'pending'">
            <BaseButton
              size="sm"
              @click="teams.decide(request.id, 'approve', request.requestedRole as TeamRole)"
            >
              {{ t('cloudTeams.approve') }}
            </BaseButton>
            <BaseButton
              size="sm"
              variant="ghost"
              @click="teams.decide(request.id, 'deny')"
            >
              {{ t('cloudTeams.deny') }}
            </BaseButton>
          </template>
        </li>
      </ul>
    </div>

    <div class="cloud-teams__block">
      <h4>{{ t('cloudTeams.grants') }}</h4>
      <ul class="cloud-teams__list">
        <li
          v-for="grant in teams.grants"
          :key="grant.id"
          class="cloud-teams__row"
        >
          <span class="cloud-teams__grow">
            {{ grant.deviceLabel }} · {{ statusLabel('cloudTeams.grantStatus', grant.status) }}
          </span>
          <BaseComboBox
            :model-value="grant.role as TeamRole"
            :options="roleOptions"
            :disabled="teams.working"
            :aria-label="t('cloudTeams.role')"
            @update:model-value="changeRole(grant, $event as TeamRole)"
          />
          <BaseButton
            v-if="grant.status !== 'revoked'"
            size="sm"
            variant="danger"
            @click="teams.revokeGrant(grant)"
          >
            {{ t('cloudTeams.revoke') }}
          </BaseButton>
        </li>
      </ul>
    </div>

    <div class="cloud-teams__block">
      <h4>{{ t('cloudTeams.sessions') }}</h4>
      <ul class="cloud-teams__list">
        <li
          v-for="session in teams.sessions"
          :key="session.id"
          class="cloud-teams__row"
        >
          <span class="cloud-teams__grow">{{ session.openedAt }}</span>
          <BaseButton
            size="sm"
            variant="danger"
            @click="teams.revokeSession(session.id)"
          >
            {{ t('cloudTeams.revoke') }}
          </BaseButton>
        </li>
      </ul>
    </div>
  </div>
</template>
