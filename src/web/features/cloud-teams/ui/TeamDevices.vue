<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { ref } from 'vue';
import { useTeamsStore } from '@/entities/cloud-teams/model/teams-store.ts';
import { computed } from 'vue';
import { useTeamLabels } from '../lib/use-team-labels.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
const { t, te } = useI18n();
const teams = useTeamsStore();
const hostDeviceLabel = ref('');
const confirmRevoke = ref<string | null>(null);
const { statusLabel } = useTeamLabels();
/** Serving needs an active host device for this team; the Cloud enforces it again. */
const hasHostDevice = computed(() => teams.devices.some((device) => device.kind === 'host' && device.status === 'active'));
const linkLabel = computed(() => {
  const state = teams.link.state;
  return te(`cloudTeams.link.${state}`) ? t(`cloudTeams.link.${state}`) : state;
});


async function revokeDevice(deviceId: string): Promise<void> {
  confirmRevoke.value = null;
  await teams.revokeDevice(deviceId);
}
</script>

<template>
  <div class="cloud-teams__device-settings">
    <div class="cloud-teams__block">
      <h4>{{ t('cloudTeams.serving') }}</h4>
      <p class="cloud-teams__note">
        {{ linkLabel }}
        <template v-if="teams.link.lastError">
          · {{ teams.link.lastError }}
        </template>
      </p>
      <div class="cloud-teams__row">
        <BaseButton
          v-if="teams.link.state === 'stopped'"
          :disabled="!hasHostDevice || teams.working"
          @click="teams.setHostLink(true)"
        >
          {{ t('cloudTeams.startServing') }}
        </BaseButton>
        <BaseButton
          v-else
          variant="ghost"
          :disabled="teams.working"
          @click="teams.setHostLink(false)"
        >
          {{ t('cloudTeams.stopServing') }}
        </BaseButton>
      </div>
      <p
        v-if="!hasHostDevice"
        class="cloud-teams__note"
      >
        {{ t('cloudTeams.needHostDevice') }}
      </p>
    </div>

    <div class="cloud-teams__block">
      <h4>{{ t('cloudTeams.devices') }}</h4>
      <form
        class="cloud-teams__row"
        @submit.prevent="teams.enroll('host', hostDeviceLabel)"
      >
        <BaseTextField
          v-model="hostDeviceLabel"
          class="cloud-teams__grow"
          :placeholder="t('cloudTeams.deviceLabel')"
          :aria-label="t('cloudTeams.deviceLabel')"
          maxlength="120"
        />
        <BaseButton
          type="submit"
          :disabled="hostDeviceLabel.trim().length === 0 || teams.working"
        >
          {{ t('cloudTeams.enrollHost') }}
        </BaseButton>
      </form>
      <ul class="cloud-teams__list">
        <li
          v-for="device in teams.devices"
          :key="device.id"
          class="cloud-teams__row"
        >
          <span class="cloud-teams__grow">
            {{ device.label }} · {{ statusLabel('cloudTeams.kind', device.kind) }} ·
            {{ statusLabel('cloudTeams.deviceStatus', device.status) }}
          </span>
          <BaseButton
            v-if="confirmRevoke !== device.id"
            size="sm"
            variant="danger"
            @click="confirmRevoke = device.id"
          >
            {{ t('cloudTeams.revoke') }}
          </BaseButton>
          <BaseButton
            v-else
            size="sm"
            variant="danger"
            @click="revokeDevice(device.id)"
          >
            {{ t('cloudTeams.confirmRevoke') }}
          </BaseButton>
        </li>
      </ul>
    </div>
  </div>
</template>
