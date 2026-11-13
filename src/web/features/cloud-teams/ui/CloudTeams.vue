<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { failureText } from '@/shared/api/cloud-failure.ts';
import { useTeamsStore } from '@/entities/cloud-teams/model/teams-store.ts';
import { useTeamLabels } from '../lib/use-team-labels.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import GuideButton from '@/features/guided-help/ui/GuideButton.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import TeamAccessForms from './TeamAccessForms.vue';
import TeamDevices from './TeamDevices.vue';
import TeamInvitations from './TeamInvitations.vue';
import TeamMembers from './TeamMembers.vue';

const { t } = useI18n();
const teams = useTeamsStore();
const mode = ref<'host' | 'join'>('host');
const tabs = computed(() => [{ value: 'host', label: t('cloudTeams.myTeams') }, { value: 'join', label: t('cloudTeams.join') }]);
const { statusLabel } = useTeamLabels();
const guideStep = computed(() => !teams.selected ? 'createTeam' : !teams.localHostDevice ? 'enrollHost' : teams.link.state !== 'ready' || teams.link.hostId !== teams.selectedId ? 'startServing' : 'invitations');
const guideSteps = computed(() => [{ id: guideStep.value, selector: `[data-guide=${guideStep.value}]`, title: t(`cloudTeams.${guideStep.value}`), description: t(`cloudTeams.guideSteps.${guideStep.value}`) }]);
const newTeamName = ref('');
const renameValue = ref('');
watch(() => teams.selected?.name, name => { renameValue.value = name ?? ''; });
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

let stopped = false;
let polling: ReturnType<typeof setTimeout> | undefined;
async function pollLink(): Promise<void> {
  if (stopped) return;
  await teams.refreshLink();
  if (!stopped) polling = setTimeout(() => { void pollLink(); }, 3000);
}
const account = useAccountStore();
onMounted(async () => {
  await account.ensureRestored();
  if (stopped) return;
  await teams.refresh();
  if (!stopped) void pollLink();
});
onBeforeUnmount(() => {
  stopped = true;
  clearTimeout(polling);
  // Invitation tokens must not outlive the panel.
  teams.reset();
});
</script>

<template>
  <section
    class="cloud-teams"
    data-guide-scope="teams"
  >
    <header class="cloud-teams__header">
      <h3 class="cloud-teams__title">
        {{ t('cloudTeams.title') }}
      </h3>
      <GuideButton
        v-if="mode === 'host'"
        id="teams-v1"
        :steps="guideSteps"
        :completed="teams.issued !== null"
      />
    </header>

    <BaseAlert
      v-if="teams.error"
      tone="error"
      :title="failureText(t, teams.errorDetail, teams.error)"
      compact
    />

    <BaseTabs
      v-model="mode"
      :tabs="tabs"
    >
      <template v-if="mode === 'host'">
        <form
          data-guide="createTeam"
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

        <p
          v-if="teams.loading"
          role="status"
          class="cloud-teams__note"
        >
          {{ t('cloudTeams.loading') }}
        </p>
        <BaseEmptyState
          v-else-if="!teams.error && teams.hosts.length === 0"
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
      </template>
      <TeamAccessForms v-else />
    </BaseTabs>
  </section>
</template>

<style src="./cloud-teams.css"></style>
