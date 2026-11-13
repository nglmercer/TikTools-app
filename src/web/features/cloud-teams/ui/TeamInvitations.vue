<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { useTeamsStore } from '@/entities/cloud-teams/model/teams-store.ts';
import { teamApi, type TeamPerson } from '@/entities/cloud-teams/api/teams-api.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import BaseSearchSelect from '@/shared/ui/search-select/BaseSearchSelect.vue';
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
const api = teamApi(useControlClient());
const recipient = ref('');
const people = ref<TeamPerson[]>([]);
const searchError = ref('');
const searching = ref(false);
const options = computed(() => people.value.map(person => ({ value: person.id, label: person.nickname, disabled: person.ambiguous, detail: person.ambiguous ? t('cloudTeams.ambiguousNickname') : undefined })));
let searchGeneration = 0;
let pendingSearch: ReturnType<typeof setTimeout> | undefined;
function search(query: string): void {
  const generation = ++searchGeneration;
  clearTimeout(pendingSearch);
  people.value = []; recipient.value = ''; searchError.value = ''; searching.value = false;
  if (query.trim().length < 3 || query.trim().length > 64 || query.includes('@')) return;
  searching.value = true;
  pendingSearch = setTimeout(async () => {
    try {
      const result = await api.people(query.trim());
      if (generation === searchGeneration) people.value = result;
    } catch (failure) { if (generation === searchGeneration) searchError.value = errorMessage(failure); }
    finally { if (generation === searchGeneration) searching.value = false; }
  }, 350);
}
watch(() => teams.selectedId, () => search(''));
onBeforeUnmount(() => { searchGeneration++; clearTimeout(pendingSearch); });
async function createInvitation(): Promise<void> {
  if (!people.value.some(person => person.id === recipient.value && !person.ambiguous)) return;
  if (await teams.createInvitation(inviteRole.value, inviteTtl.value, recipient.value)) {
    copied.value = false;
    search('');
  }
}
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
  <div
    data-guide="invitations"
    class="cloud-teams__block"
  >
    <h4>{{ t('cloudTeams.invitations') }}</h4>
    <div class="cloud-teams__row">
      <BaseSearchSelect
        v-model="recipient"
        class="cloud-teams__grow"
        :options="options"
        :placeholder="t('cloudTeams.searchPeople')"
        :aria-label="t('cloudTeams.searchPeople')"
        :disabled="teams.working"
        :aria-busy="searching"
        @search="search"
      />
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
        :disabled="teams.working || !people.some(person => person.id === recipient && !person.ambiguous)"
        @click="createInvitation"
      >
        {{ t('cloudTeams.createInvitation') }}
      </BaseButton>
    </div>
    <BaseAlert
      v-if="searchError"
      tone="error"
      :title="searchError"
      compact
    />
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
