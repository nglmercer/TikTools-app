<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { ref } from 'vue';
import { useTeamsStore } from '@/entities/cloud-teams/model/teams-store.ts';
import { computed } from 'vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
const { t } = useI18n();
const teams = useTeamsStore();
const joinToken = ref('');
const askTeam = ref('');
const askOperation = ref('live.status');
const controllerLabel = ref('');
/** Read-only operations a host answers today; the host still checks the grant at execution. */
const askOperations = ['live.status', 'workflow.list', 'automation.list', 'competition.list', 'points.read'] as const;
const operationLabels = {"live.status": "liveStatus", "workflow.list": "workflowList", "automation.list": "automationList", "competition.list": "competitionList", "points.read": "pointsRead"} as const;
const askOptions = computed(() =>
  askOperations.map((operation) => ({ value: operation, label: t(`cloudTeams.operations.${operationLabels[operation]}`) })),
);
async function askLiveStatus(): Promise<void> {
  const hostId = askTeam.value.trim();
  if (hostId) await teams.runCommand(hostId, askOperation.value);
}

async function joinWithToken(): Promise<void> {
  const token = joinToken.value.trim();
  if (!token) return;
  if (await teams.redeem(token)) joinToken.value = '';
}


</script>

<template>
  <div class="cloud-teams__access">
    <form
      class="cloud-teams__block"
      @submit.prevent="joinWithToken"
    >
      <h4>{{ t('cloudTeams.join') }}</h4>
      <p class="cloud-teams__note">
        {{ t('cloudTeams.joinHint') }}
      </p>
      <div class="cloud-teams__row">
        <BaseTextField
          v-model="joinToken"
          class="cloud-teams__grow"
          :placeholder="t('cloudTeams.joinToken')"
          :aria-label="t('cloudTeams.joinToken')"
        />
        <BaseButton
          type="submit"
          :disabled="joinToken.trim().length === 0 || teams.working"
        >
          {{ t('cloudTeams.joinButton') }}
        </BaseButton>
      </div>
      <p
        v-if="teams.lastRedeem"
        class="cloud-teams__note"
      >
        {{ t('cloudTeams.joinRequested', { role: t(`cloudTeams.roles.${teams.lastRedeem.requestedRole}`) }) }}
      </p>
    </form>

    <form
      class="cloud-teams__block"
      @submit.prevent="askLiveStatus"
    >
      <h4>{{ t('cloudTeams.ask') }}</h4>
      <p class="cloud-teams__note">
        {{ t('cloudTeams.askHint') }}
      </p>
      <div class="cloud-teams__row">
        <BaseTextField
          v-model="askTeam"
          class="cloud-teams__grow"
          :placeholder="t('cloudTeams.askTeam')"
          :aria-label="t('cloudTeams.askTeam')"
        />
        <BaseComboBox
          v-model="askOperation"
          :options="askOptions"
          :aria-label="t('cloudTeams.operation')"
        />
        <BaseButton
          type="submit"
          :disabled="askTeam.trim().length === 0 || teams.working"
        >
          {{ t('cloudTeams.send') }}
        </BaseButton>
      </div>
      <p
        v-if="teams.lastCommand"
        class="cloud-teams__note"
      >
        {{ t(`cloudTeams.outcome.${teams.lastCommand.status}`) }}
        <template v-if="teams.lastCommand.code">
          · {{ teams.lastCommand.code }}
        </template>
      </p>
      <BaseJsonView
        v-if="teams.lastCommand?.result"
        :value="teams.lastCommand.result"
        max-height="300px"
      />
    </form>

    <div class="cloud-teams__block">
      <h4>{{ t('cloudTeams.thisPc') }}</h4>
      <form
        class="cloud-teams__row"
        @submit.prevent="teams.enroll('controller', controllerLabel)"
      >
        <BaseTextField
          v-model="controllerLabel"
          class="cloud-teams__grow"
          :placeholder="t('cloudTeams.deviceLabel')"
          :aria-label="t('cloudTeams.deviceLabel')"
          maxlength="120"
        />
        <BaseButton
          type="submit"
          :disabled="controllerLabel.trim().length === 0 || teams.working"
        >
          {{ t('cloudTeams.enrollController') }}
        </BaseButton>
      </form>
    </div>
  </div>
</template>
