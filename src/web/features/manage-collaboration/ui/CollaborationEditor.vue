<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { createCollaborationOwner } from '@/entities/collaboration/model/owner.ts';
import { viewerGrant, type Configuration, type Grant, type PairingRequest, type Principal } from '@/entities/collaboration/api/types.ts';
import { parseGrant, prepareConfiguration, splitIds } from '@/entities/collaboration/lib/grants.ts';
import { activeGateway } from '@/entities/collaboration/lib/gateway.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import { areaById, type AccessArea } from '../model/access-areas.ts';
import { cloneGrant } from '../model/clone-grant.ts';
import type { CollaborationConfirmation } from '../model/confirmation.ts';
import { presetGrant, type Preset } from '../model/presets.ts';
import { useCollaborationPresentation } from '../lib/use-collaboration-presentation.ts';
import AccessInspector from './AccessInspector.vue';
import AccessMatrix, { type MatrixSelection } from './AccessMatrix.vue';
import ConnectionSettings from './ConnectionSettings.vue';
import InvitationDialog from './InvitationDialog.vue';

interface Draft { revision: string; grant: Grant; original: string }
const { t } = useI18n();
const emit = defineEmits<{ teams: [] }>();
const { listenerStatus } = useCollaborationPresentation();
const view = ref<'access' | 'connection'>('access');
const owner = createCollaborationOwner(useControlClient());
owner.watchPeople();
const { status, people, working, error, uncertain, connected, secret, canInvite, joinLink } = owner;
const gateway = computed(() => status.value ? activeGateway(status.value) : null);
const native = computed(() => !!status.value?.peerGateway?.available);
const draft = ref<Configuration | null>(null);
const baseRevision = ref('');
const allowedOrigins = ref('');
const message = ref('');
const newGrant = ref<Grant>(viewerGrant());
// A grant that was issued, kept so the dialog still describes it after the draft is edited again.
const issuedGrant = ref<Grant>(viewerGrant());
const drafts = ref<Record<string, Draft>>({});
const selection = ref<MatrixSelection | null>(null);

const confirmation = ref<CollaborationConfirmation | null>(null);
const confirmOpen = computed({ get: () => confirmation.value !== null, set: (value) => { if (!value) confirmation.value = null; } });
const blocked = computed(() => working.value || !connected.value || uncertain.value);
const changed = computed(() => !!status.value && baseRevision.value !== status.value.configurationRevision);
// Unsaved edits to the connection, compared with what the host last reported.
const dirty = computed(() => {
  if (!draft.value || !status.value) return false;
  const saved = status.value.configuration;
  return JSON.stringify({ ...draft.value, allowedOrigins: splitIds(allowedOrigins.value) }) !== JSON.stringify({ ...saved, allowedOrigins: saved.allowedOrigins });
});
const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
function resetDraft(): void {
  if (!status.value) return;
  draft.value = clone(status.value.configuration); baseRevision.value = status.value.configurationRevision;
  allowedOrigins.value = draft.value.allowedOrigins.join('\n'); message.value = '';
}
watch(status, () => { if (!draft.value) resetDraft(); });

const pending = computed(() => people.value.requests.filter(request => request.state === 'pending' && request.expiresAt > owner.now.value));
const principals = computed(() => people.value.principals.filter(person => !person.revoked));
const online = computed(() => {
  const counts: Record<string, number> = {};
  const connections = gateway.value?.sessionConnections ?? {};
  for (const session of people.value.sessions) if (!session.revoked && connections[session.sessionId]) counts[session.principalId] = (counts[session.principalId] ?? 0) + connections[session.sessionId]!;
  return counts;
});
const listener = computed(() => status.value ? listenerStatus(status.value) : null);

async function applyConfiguration(): Promise<void> {
  if (!draft.value || changed.value || blocked.value) return;
  message.value = '';
  try {
    const input = prepareConfiguration({ ...clone(draft.value), allowedOrigins: splitIds(allowedOrigins.value) });
    if (status.value?.configuration.enabled && !input.enabled) { confirmation.value = { action: 'disable', configuration: input, revision: baseRevision.value }; return; }
    if (await owner.configure(input, baseRevision.value)) { resetDraft(); message.value = t('collaboration.saved'); }
    else if (owner.errorCode.value === 'revision_conflict') message.value = t('collaboration.changed');
  } catch { message.value = t('collaboration.invalidConfiguration'); }
}
async function createInvitation(): Promise<void> {
  message.value = '';
  try {
    const grant = parseGrant(newGrant.value);
    issuedGrant.value = cloneGrant(grant);
    await owner.createInvitation(grant);
  } catch { message.value = t('collaboration.invalidGrant'); }
}
function applyPreset(kind: Preset): void { newGrant.value = presetGrant(kind); }

let selectionToken = 0;
async function select(target: string, area: AccessArea['id']): Promise<void> {
  const token = ++selectionToken;
  selection.value = { target, area };
  if (target === 'new' || drafts.value[target]) return;
  const principal = await owner.getPrincipal(target);
  if (token !== selectionToken || !principal || principal.revoked) return;
  drafts.value[target] = { revision: principal.grantRevision, grant: cloneGrant(principal.grant), original: JSON.stringify(principal.grant) };
}
const selectedArea = computed(() => selection.value ? areaById(selection.value.area) : null);
const selectedPerson = computed<Principal | undefined>(() => principals.value.find(person => person.principalId === selection.value?.target));
const inspectorTitle = computed(() => selection.value?.target === 'new' ? t('collaboration.newInvitation') : selectedPerson.value?.clientLabel ?? '');
// Edits go to the identity's own draft; the first selection only needs to wait for the latest revision to load.
const inspectorReady = computed(() => selection.value?.target === 'new' || (!!selection.value && !!drafts.value[selection.value.target]));
const inspectorGrant = computed<Grant>({
  get: () => selection.value?.target === 'new' ? newGrant.value : drafts.value[selection.value?.target ?? '']?.grant ?? selectedPerson.value?.grant ?? viewerGrant(),
  set: (value) => {
    if (!selection.value) return;
    if (selection.value.target === 'new') newGrant.value = value;
    else if (drafts.value[selection.value.target]) drafts.value[selection.value.target]!.grant = value;
  },
});
const draftDirty = (id: string): boolean => !!drafts.value[id] && JSON.stringify(drafts.value[id]!.grant) !== drafts.value[id]!.original;
const visibleDrafts = computed(() => Object.fromEntries(Object.entries(drafts.value).filter(([id]) => draftDirty(id))));
function discard(id: string): void { delete drafts.value[id]; }
function confirmGrant(id: string): void {
  const item = drafts.value[id]; if (!item) return;
  try { confirmation.value = { action: 'grant', id, revision: item.revision, grant: parseGrant(item.grant) }; }
  catch { message.value = t('collaboration.invalidGrant'); }
}
function requestApprove(request: PairingRequest): void { confirmation.value = { action: 'approve', id: request.requestId, request, grant: request.grant }; }
function requestReject(request: PairingRequest): void { confirmation.value = { action: 'reject', id: request.requestId }; }
function requestRevoke(person: Principal): void { confirmation.value = { action: 'principal', id: person.principalId }; }
function requestRevokeInvite(inviteId: string): void { confirmation.value = { action: 'invite', id: inviteId }; }
const confirmTitle = computed(() => t(`collaboration.${confirmation.value?.action === 'disable' ? 'disableTitle' : confirmation.value?.action === 'approve' ? 'approveTitle' : confirmation.value?.action === 'grant' ? 'grantTitle' : confirmation.value?.action === 'reject' ? 'rejectTitle' : 'revokeTitle'}`));
const confirmMessage = computed(() => {
  const keys = { disable: 'disableMessage', approve: 'approveMessage', reject: 'rejectMessage', grant: 'grantMessage', session: 'revokeSessionMessage', principal: 'revokePrincipalMessage', invite: 'revokeInviteMessage' } as const;
  return t(`collaboration.${keys[confirmation.value?.action ?? 'session']}`);
});
async function confirm(): Promise<void> {
  const item = confirmation.value; if (!item || blocked.value) return;
  let success = false;
  if (item.action === 'disable' && item.configuration && item.revision) success = await owner.configure(item.configuration, item.revision);
  else if (item.action === 'grant' && item.id && item.revision && item.grant) success = await owner.setGrant(item.id, item.revision, item.grant);
  else if (item.id) {
    const actions = { approve: 'requests.approve', reject: 'requests.reject', session: 'sessions.revoke', principal: 'principals.revoke', invite: 'invites.revoke' } as const;
    if (item.action in actions) success = await owner.act(actions[item.action as keyof typeof actions], item.id);
  }
  if (uncertain.value) { confirmation.value = null; return; }
  if (success) {
    confirmation.value = null;
    if (item.action === 'disable') resetDraft();
    if (item.action === 'grant' && item.id) discard(item.id);
    if (item.action === 'principal' && item.id) { discard(item.id); if (selection.value?.target === item.id) selection.value = null; }
  } else if (owner.errorCode.value === 'revision_conflict') { confirmation.value = null; message.value = t(item.action === 'grant' ? 'collaboration.grantChanged' : 'collaboration.changed'); }
}
onMounted(() => { void owner.start(); });
onUnmounted(() => { owner.stop(); confirmation.value = null; drafts.value = {}; });
</script>

<template>
  <div class="collaboration-editor">
    <BaseAlert
      v-if="error"
      :title="error"
    />
    <BaseAlert
      v-if="message"
      tone="info"
      :title="message"
    />
    <BaseAlert
      v-if="!connected && status"
      tone="warning"
      :title="t('collaboration.disconnected')"
    />
    <BaseAlert
      v-if="!status && !error"
      tone="info"
      :title="t('collaboration.loading')"
    />
    <BaseGroupBox
      v-if="uncertain"
      :title="t('collaboration.uncertain')"
    >
      <BaseButton @click="uncertain = false">
        {{ t('collaboration.acknowledge') }}
      </BaseButton>
    </BaseGroupBox>
    <template v-if="status && draft">
      <BaseAlert
        v-if="!gateway?.available"
        tone="warning"
        :title="t('collaboration.unavailable')"
      />
      <BaseAlert
        v-if="changed"
        tone="warning"
        :title="t('collaboration.changed')"
      />
      <template v-if="view === 'access'">
        <header class="collaboration-editor__header">
          <div>
            <h2>{{ t('collaboration.accessTitle') }}</h2>
            <p class="collaboration-editor__muted">
              {{ t('collaboration.accessHint') }}
            </p>
          </div>
          <div class="collaboration-editor__status">
            <BaseStatus
              v-if="listener"
              :label="listener.label"
              :tone="listener.tone"
            />
            <span class="collaboration-editor__muted">{{ native ? t('cloudTeams.title') : (gateway?.address ?? status.configuration.bindAddress) }} · {{ t('collaboration.connected', { count: gateway?.connections ?? 0 }) }}</span>
            <BaseButton
              v-if="native"
              @click="emit('teams')"
            >
              {{ t('cloudTeams.title') }}
            </BaseButton>
            <BaseButton @click="view = 'connection'">
              {{ t('collaboration.connectionSettings') }}
            </BaseButton>
          </div>
        </header>
        <AccessMatrix
          :requests="pending"
          :principals="principals"
          :online="online"
          :drafts="visibleDrafts"
          :new-grant="newGrant"
          :selection="selection"
          :blocked="blocked"
          :can-invite="canInvite"
          @select="select"
          @preset="applyPreset"
          @approve="requestApprove"
          @reject="requestReject"
          @revoke="requestRevoke"
          @create="createInvitation"
          @connection="view = 'connection'"
        />
        <AccessInspector
          v-if="selection && selectedArea && inspectorReady"
          v-model="inspectorGrant"
          :title="inspectorTitle"
          :area="selectedArea"
          :reset-key="selection.target"
          :disabled="blocked"
          :saveable="selection.target !== 'new'"
          :dirty="selection.target !== 'new' && draftDirty(selection.target)"
          @apply="confirmGrant(selection.target)"
          @discard="discard(selection.target)"
        />
        <p
          v-else
          class="collaboration-editor__muted"
        >
          {{ t('collaboration.selectCell') }}
        </p>
      </template>
      <template v-else>
        <header class="collaboration-editor__header collaboration-editor__header--back">
          <BaseButton @click="view = 'access'">
            {{ t('collaboration.backToAccess') }}
          </BaseButton>
          <h2>{{ t('collaboration.connectionTitle') }}</h2>
        </header>
        <ConnectionSettings
          v-model="draft"
          v-model:allowed-origins="allowedOrigins"
          :status="status"
          :blocked="blocked"
          :changed="changed"
          :dirty="dirty"
          :working="working"
          :available="gateway?.available ?? false"
          @apply="applyConfiguration"
          @reset="resetDraft"
          @teams="emit('teams')"
        />
      </template>
    </template>
    <InvitationDialog
      :secret="secret"
      :join-link="joinLink"
      :grant="issuedGrant"
      :fingerprint="status?.gateway.certificateFingerprint"
      :blocked="blocked"
      @close="secret = null"
      @revoke="requestRevokeInvite"
      @error="message = $event"
    />
    <BaseConfirmDialog
      v-model:open="confirmOpen"
      :title="confirmTitle"
      :message="confirmMessage"
      :confirm-label="t('common.apply')"
      :busy="working"
      :danger="confirmation?.action !== 'approve' && confirmation?.action !== 'grant'"
      @confirm="confirm"
    >
      <BaseAlert
        v-if="error"
        :title="error"
      />
      <code v-if="confirmation?.id">{{ confirmation.id }}</code>
      <p v-if="confirmation?.request">
        {{ confirmation.request.clientLabel }} · {{ confirmation.request.clientKind }}
      </p>
      <BaseJsonView
        v-if="confirmation?.grant"
        :value="confirmation.grant"
        max-height="300px"
      />
    </BaseConfirmDialog>
  </div>
</template>

<style src="./collaboration.css"></style>
