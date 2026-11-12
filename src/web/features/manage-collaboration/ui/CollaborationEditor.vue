<script setup lang="ts">
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { createCollaborationOwner } from '@/entities/collaboration/model/owner.ts';
import { viewerGrant, type AdminTab, type Configuration, type Grant, type Principal } from '@/entities/collaboration/api/types.ts';
import { parseGrant, prepareConfiguration, splitIds } from '@/entities/collaboration/lib/grants.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';
import GrantEditor from './GrantEditor.vue';
import type { CollaborationConfirmation } from '../model/confirmation.ts';
import CollaborationOverview from './CollaborationOverview.vue';
import DirectConnectionForm from './DirectConnectionForm.vue';
import CollaborationRecords from './CollaborationRecords.vue';
import CollaborationInvitation from './CollaborationInvitation.vue';

const { t } = useI18n();
const area = ref<'overview' | 'people' | 'invite' | 'direct'>('overview');
// Listen address, public URL, TLS and allowed origins belong to the direct connection, not to the normal flow.
const areaTabs = computed(() => [
 { value: 'overview' as const, label: t('settingsLayout.overview') },
 { value: 'people' as const, label: t('settingsLayout.people') },
  { value: 'invite' as const, label: t('collaboration.invites') },
 { value: 'direct' as const, label: t('collaboration.directTransport') },
]);
const owner = createCollaborationOwner(useControlClient());
const { status, pages, tab, loading, working, error, uncertain, connected, checkedAt, secret, canInvite, joinLink, next, previous } = owner;
const draft = ref<Configuration | null>(null);
const baseRevision = ref('');
const allowedOrigins = ref('');
const message = ref('');
const grant = ref(viewerGrant());
const principalDraft = ref<{ id: string; revision: string; grant: Grant } | null>(null);

const confirmation = ref<CollaborationConfirmation | null>(null);
const confirmOpen = computed({ get: () => confirmation.value !== null, set: (value) => { if (!value) confirmation.value = null; } });
const blocked = computed(() => working.value || !connected.value || uncertain.value);
const changed = computed(() => !!status.value && baseRevision.value !== status.value.configurationRevision);
const adminTab = computed({ get: () => tab.value, set: (value: AdminTab) => { void owner.selectTab(value); } });
const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
function resetDraft(): void {
  if (!status.value) return;
  draft.value = clone(status.value.configuration); baseRevision.value = status.value.configurationRevision;
  allowedOrigins.value = draft.value.allowedOrigins.join('\n'); message.value = '';
}
watch(status, () => { if (!draft.value) resetDraft(); });
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
  try { await owner.createInvitation(parseGrant(grant.value)); } catch { message.value = t('collaboration.invalidGrant'); }
}
let principalSelection = 0;
async function editPrincipal(input: Principal | string): Promise<void> {
  const selection = ++principalSelection;
  const principal = await owner.getPrincipal(typeof input === 'string' ? input : input.principalId);
  if (principal && selection === principalSelection && !principal.revoked) principalDraft.value = { id: principal.principalId, revision: principal.grantRevision, grant: clone(principal.grant) };
}
function confirmGrant(): void {
  if (!principalDraft.value) return;
  try { confirmation.value = { action: 'grant', id: principalDraft.value.id, revision: principalDraft.value.revision, grant: parseGrant(principalDraft.value.grant) }; }
  catch { message.value = t('collaboration.invalidGrant'); }
}
function requestDisable(): void { if (status.value) confirmation.value = { action: 'disable', configuration: { ...clone(status.value.configuration), enabled: false }, revision: status.value.configurationRevision }; }
function closePrincipal(): void { principalSelection += 1; principalDraft.value = null; }
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
  if (success) { confirmation.value = null; if (item.action === 'disable') resetDraft(); if (item.action === 'grant') principalDraft.value = null; }
  else if (owner.errorCode.value === 'revision_conflict') { confirmation.value = null; message.value = t(item.action === 'grant' ? 'collaboration.grantChanged' : 'collaboration.changed'); }
}
onMounted(() => { void owner.start(); });
onUnmounted(() => { owner.stop(); confirmation.value = null; principalDraft.value = null; });
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
      <BaseTabs
        v-model="area"
        :tabs="areaTabs"
        :label="t('collaboration.title')"
        class="collaboration-editor__navigation"
      >
        <BaseAlert
          v-if="!status.gateway.available"
          tone="warning"
          :title="t('collaboration.unavailable')"
        />
        <BaseAlert
          v-if="changed"
          tone="warning"
          :title="t('collaboration.changed')"
        />
        <BaseButton
          v-if="status.pendingRequests > 0 && area !== 'people'"
          class="collaboration-editor__pending"
          @click="area = 'people'; tab = 'requests'"
        >
          {{ t('settingsLayout.requests') }} · {{ status.pendingRequests }}
        </BaseButton>
        <CollaborationOverview
          v-show="area === 'overview'"
          :status="status"
          :checked-at="checkedAt"
          :blocked="blocked"
          @disable="requestDisable"
        />
        <DirectConnectionForm
          v-show="area === 'direct'"
          v-model="draft"
          v-model:allowed-origins="allowedOrigins"
          :blocked="blocked"
          :changed="changed"
          :working="working"
          :available="status.gateway.available"
          @apply="applyConfiguration"
          @reset="resetDraft"
        />
        <CollaborationRecords
          v-show="area === 'people'"
          v-model:tab="adminTab"
          :pages="pages"
          :loading="loading"
          :blocked="blocked"
          :has-previous="previous.length > 0"
          :has-next="next !== null"
          :now="owner.now.value"
          :session-connections="status.gateway.sessionConnections ?? {}"
          @refresh="owner.first"
          @previous="owner.previousPage"
          @next="owner.nextPage"
          @edit="editPrincipal"
          @confirm="confirmation = clone($event)"
        />
        <CollaborationInvitation
          v-show="area === 'invite'"
          v-model="grant"
          :blocked="blocked"
          :can-invite="canInvite"
          :join-link="joinLink"
          :secret="secret"
          @create="createInvitation"
          @hide="secret = null"
          @error="message = $event"
        />
        <BaseGroupBox
          v-if="principalDraft"
          :title="t('collaboration.editGrant')"
        >
          <p class="collaboration-editor__meta">
            <code>{{ principalDraft.id }}</code> · {{ t('collaboration.revision') }} {{ principalDraft.revision }}
          </p>
          <GrantEditor
            :key="`${principalDraft.id}:${principalDraft.revision}`"
            v-model="principalDraft.grant"
            :disabled="blocked"
          />
          <div class="collaboration-editor__actions">
            <BaseButton
              :disabled="blocked"
              variant="primary"
              @click="confirmGrant"
            >
              {{ t('collaboration.grantSave') }}
            </BaseButton>
            <BaseButton @click="closePrincipal">
              {{ t('common.cancel') }}
            </BaseButton>
          </div>
        </BaseGroupBox>
      </BaseTabs>
    </template>
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
