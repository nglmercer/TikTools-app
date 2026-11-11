<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { PRESET_ROLES, type TeamGrant, type TeamRole } from '@/entities/cloud-teams/api/teams-api.ts';
import { useTeamsStore } from '@/entities/cloud-teams/model/teams-store.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

const { t, te } = useI18n();
const teams = useTeamsStore();

const newTeamName = ref('');
const renameValue = ref('');
const controllerLabel = ref('');
const joinToken = ref('');
const askTeam = ref('');
const hostDeviceLabel = ref('');
const inviteRole = ref<TeamRole>('observer');
const inviteTtl = ref<number>(900);
const askOperation = ref('live.status');
const copied = ref(false);
const confirmRevoke = ref<string | null>(null);

const roleOptions = computed(() =>
  PRESET_ROLES.map((role) => ({ value: role, label: t(`cloudTeams.roles.${role}`) })),
);
const ttlOptions = computed(() => [
  { value: 300, label: t('cloudTeams.ttl.minutes', { count: 5 }) },
  { value: 900, label: t('cloudTeams.ttl.minutes', { count: 15 }) },
]);
/** Read-only operations a host answers today; the host still checks the grant at execution. */
const askOperations = ['live.status', 'workflow.list', 'automation.list', 'competition.list', 'points.read'] as const;
const askOptions = computed(() =>
  askOperations.map((operation) => ({ value: operation, label: t(`cloudTeams.operations.${operation}`) })),
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

const canCreateTeam = computed(() => newTeamName.value.trim().length > 0 && !teams.working);
/** Serving needs an active host device for this team; the Cloud enforces it again. */
const hasHostDevice = computed(() => teams.devices.some((device) => device.kind === 'host' && device.status === 'active'));
const linkLabel = computed(() => {
  const state = teams.link.state;
  return te(`cloudTeams.link.${state}`) ? t(`cloudTeams.link.${state}`) : state;
});

/** Known statuses are translated; anything else is shown as the server sent it. */
function statusLabel(prefix: string, value: string): string {
  const key = `${prefix}.${value}`;
  return te(key) ? t(key) : value;
}

async function createTeam(): Promise<void> {
  if (!canCreateTeam.value) return;
  const created = await teams.createTeam(newTeamName.value);
  if (created) newTeamName.value = '';
}

async function selectTeam(id: string): Promise<void> {
  await teams.select(id);
  renameValue.value = teams.selected?.name ?? '';
}

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

function changeRole(grant: TeamGrant, role: TeamRole): void {
  if (grant.role !== role) void teams.changeRole(grant, role);
}

async function revokeDevice(deviceId: string): Promise<void> {
  confirmRevoke.value = null;
  await teams.revokeDevice(deviceId);
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
      :title="teams.error"
      compact
    />

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
      <pre
        v-if="teams.lastCommand?.result"
        class="cloud-teams__code"
      >{{ JSON.stringify(teams.lastCommand.result, null, 2) }}</pre>
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
    </section>
  </section>
</template>

<style scoped>
.cloud-teams { display:grid; gap:var(--space-3); min-width:0; }
.cloud-teams__title, .cloud-teams__block h4 { margin:0; }
.cloud-teams__hint, .cloud-teams__note, .cloud-teams__meta { color:var(--color-text-muted); margin:0; }
.cloud-teams__meta { font-size:var(--font-size-sm); display:block; }
.cloud-teams__row { display:flex; flex-wrap:wrap; gap:var(--space-2); align-items:center; min-width:0; }
.cloud-teams__grow { flex:1 1 180px; min-width:0; overflow-wrap:anywhere; }
.cloud-teams__block { display:grid; gap:var(--space-2); padding:var(--space-3); border-radius:var(--radius-md); border:1px solid var(--color-border); min-width:0; }
.cloud-teams__teams, .cloud-teams__list { list-style:none; margin:0; padding:0; display:grid; gap:var(--space-2); }
.cloud-teams__detail { display:grid; gap:var(--space-3); min-width:0; }
.cloud-teams__code { display:block; overflow-wrap:anywhere; padding:var(--space-2); border-radius:var(--radius-sm); background:var(--color-surface-raised); }
</style>
