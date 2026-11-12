<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { failureText } from '@/shared/api/cloud-failure.ts';
import { useTeamsStore } from '@/entities/cloud-teams/model/teams-store.ts';
import { useTeamLabels } from '../lib/use-team-labels.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import TeamAccessForms from './TeamAccessForms.vue';
import TeamDevices from './TeamDevices.vue';
import TeamInvitations from './TeamInvitations.vue';
import TeamMembers from './TeamMembers.vue';

const { t } = useI18n();
const teams = useTeamsStore();
const { statusLabel } = useTeamLabels();
const newTeamName = ref('');
const renameValue = ref('');
const canCreateTeam = computed(() => newTeamName.value.trim().length > 0 && !teams.working);
async function createTeam(): Promise<void> {
  if (!canCreateTeam.value) return;
  const created = await teams.createTeam(newTeamName.value);
  if (created) newTeamName.value = '';
}

async function selectTeam(id: string): Promise<void> {
  await teams.select(id);
  renameValue.value = teams.selected?.name ?? '';
}

onMounted(() => {
  void teams.refresh();
});
onBeforeUnmount(() => {
  // Invitation tokens must not outlive the panel.
  teams.reset();
});
</script>

<template>
  <section class="cloud-teams">
    <h3 class="cloud-teams__title">
      {{ t('cloudTeams.title') }}
    </h3>
    <p class="cloud-teams__hint">
      {{ t('cloudTeams.hint') }}
    </p>

    <BaseAlert
      v-if="teams.error"
      tone="error"
      :title="failureText(t, teams.errorDetail, teams.error)"
      compact
    />

    <TeamAccessForms />

    <form
      class="cloud-teams__row"
      @submit.prevent="createTeam"
    >
      <BaseTextField
        v-model="newTeamName"
        class="cloud-teams__grow"
        :placeholder="t('cloudTeams.teamName')"
        :aria-label="t('cloudTeams.teamName')"
        maxlength="80"
      />
      <BaseButton
        type="submit"
        variant="primary"
        :disabled="!canCreateTeam"
      >
        {{ t('cloudTeams.createTeam') }}
      </BaseButton>
    </form>

    <BaseEmptyState
      v-if="!teams.loading && teams.hosts.length === 0"
      :title="t('cloudTeams.empty')"
    />

    <ul
      v-else
      class="cloud-teams__teams"
    >
      <li
        v-for="team in teams.hosts"
        :key="team.id"
      >
        <BaseButton
          :pressed="teams.selectedId === team.id"
          block
          @click="selectTeam(team.id)"
        >
          {{ team.name }}
          <span class="cloud-teams__meta">
            {{ statusLabel('cloudTeams.teamStatus', team.status) }} ·
            {{ team.online ? t('cloudTeams.online') : t('cloudTeams.offline') }}
          </span>
        </BaseButton>
      </li>
    </ul>

    <section
      v-if="teams.selected"
      class="cloud-teams__detail"
      :aria-label="teams.selected.name"
    >
      <form
        class="cloud-teams__row"
        @submit.prevent="teams.renameTeam(renameValue)"
      >
        <BaseTextField
          v-model="renameValue"
          class="cloud-teams__grow"
          :aria-label="t('cloudTeams.teamName')"
          maxlength="80"
        />
        <BaseButton
          type="submit"
          :disabled="renameValue.trim().length === 0 || renameValue.trim() === teams.selected.name || teams.working"
        >
          {{ t('cloudTeams.rename') }}
        </BaseButton>
      </form>

      <TeamDevices />
      <TeamInvitations />
      <TeamMembers />
    </section>
  </section>
</template>

<style src="./cloud-teams.css"></style>
