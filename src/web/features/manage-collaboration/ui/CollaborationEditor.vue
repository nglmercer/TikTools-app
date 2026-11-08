<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { createCollaborationOwner } from '@/entities/collaboration/model/owner.ts';
import { viewerGrant, type AdminTab, type Configuration, type Grant, type PairingRequest, type Principal, type RemoteLimits } from '@/entities/collaboration/api/types.ts';
import { parseGrant, prepareConfiguration, splitIds } from '@/entities/collaboration/lib/grants.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseTextArea from '@/shared/ui/text-area/BaseTextArea.vue';
import GrantEditor from './GrantEditor.vue';
import InvitationQr from './InvitationQr.vue';

const { t, te } = useI18n();
const owner = createCollaborationOwner(useControlClient());
const { status, pages, tab, loading, working, error, uncertain, connected, checkedAt, secret, canInvite, joinLink, next, previous } = owner;
const draft = ref<Configuration | null>(null);
const baseRevision = ref('');
const allowedOrigins = ref('');
const message = ref('');
const grant = ref(viewerGrant());
const showQr = ref(false);
const copied = ref(false);
const principalDraft = ref<{ id: string; revision: string; grant: Grant } | null>(null);
type Confirmation = { action: 'disable' | 'approve' | 'reject' | 'session' | 'principal' | 'invite' | 'grant'; id?: string; grant?: Grant; revision?: string; request?: PairingRequest; configuration?: Configuration };
const confirmation = ref<Confirmation | null>(null);
const confirmOpen = computed({ get: () => confirmation.value !== null, set: (value) => { if (!value) confirmation.value = null; } });
const blocked = computed(() => working.value || !connected.value || uncertain.value);
const changed = computed(() => !!status.value && baseRevision.value !== status.value.configurationRevision);
const tabs: AdminTab[] = ['requests', 'principals', 'sessions', 'invitations', 'audit'];
const limits: Array<{ key: keyof RemoteLimits; min: number; max: number }> = [
  { key: 'connections', min: 1, max: 8 }, { key: 'connectionsPerPrincipal', min: 1, max: 4 },
  { key: 'frameBytes', min: 4096, max: 1048576 }, { key: 'paramsBytes', min: 1024, max: 262144 },
  { key: 'pendingCalls', min: 1, max: 32 }, { key: 'requestsPerSecond', min: 1, max: 30 },
  { key: 'requestBurst', min: 1, max: 60 }, { key: 'commandIdsPerPrincipal', min: 1, max: 100000 },
];
const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
function setText(key: 'bindAddress' | 'advertisedOrigin' | 'tlsCertificate' | 'tlsPrivateKey', value: string): void {
  if (!draft.value) return;
  if (key === 'bindAddress') draft.value.bindAddress = value;
  else draft.value[key] = value || null;
}
function setLimit(key: keyof RemoteLimits, event: Event): void { if (draft.value) draft.value.limits[key] = (event.target as HTMLInputElement).valueAsNumber; }
const connections = (id: string): number => status.value?.gateway.sessionConnections?.[id] ?? 0;
function setBoolean(key: 'enabled' | 'tlsProxy', value: boolean): void { if (draft.value) draft.value[key] = value; }

function resetDraft(): void {
  if (!status.value) return;
  draft.value = clone(status.value.configuration); baseRevision.value = status.value.configurationRevision;
  allowedOrigins.value = draft.value.allowedOrigins.join('\n'); message.value = '';
}
watch(status, () => { if (!draft.value) resetDraft(); });
watch(secret, () => { showQr.value = false; copied.value = false; });
const date = (value: number | null) => value === null ? '—' : new Date(value).toLocaleString();
const state = (value: string) => te(`collaboration.states.${value}`) ? t(`collaboration.states.${value}`) : value;
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
async function copyLink(): Promise<void> {
  try { await navigator.clipboard.writeText(joinLink.value); copied.value = true; } catch { message.value = t('collaboration.clipboard'); }
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
      <BaseGroupBox
        :title="t('collaboration.title')"
        :description="t('collaboration.description')"
      >
        <p>{{ t('collaboration.host') }}: <code>{{ status.hostId }}</code></p>
        <p>{{ t('collaboration.instance') }}: <code>{{ status.hostInstanceId }}</code></p>
        <p>{{ t('collaboration.connected', { count: status.gateway.connections }) }} · {{ t('collaboration.active', { count: status.activeSessions }) }} · {{ t('collaboration.pending', { count: status.pendingRequests }) }}</p>
        <p>{{ status.gateway.running ? status.configuration.enabled ? t('collaboration.running') : t('collaboration.applying') : t('collaboration.stopped') }}</p>
        <p v-if="status.configuration.enabled && status.gateway.configurationRevision !== status.configurationRevision">
          {{ t('collaboration.applying') }}
        </p>
        <p v-if="checkedAt">
          {{ t('collaboration.checked', { when: date(checkedAt) }) }}
        </p>
        <BaseAlert
          v-if="status.gateway.lastError"
          :title="status.gateway.lastError"
        />
        <p v-if="status.gateway.running && status.gateway.certificateFingerprint">
          {{ t('collaboration.fingerprint') }}: <code class="collaboration-editor__fingerprint">{{ status.gateway.certificateFingerprint }}</code>
        </p>
        <p v-else-if="status.configuration.tlsProxy">
          {{ t('collaboration.proxyFingerprint') }}
        </p>
        <BaseButton
          v-if="status.configuration.enabled"
          variant="danger"
          :disabled="blocked"
          @click="requestDisable"
        >
          {{ t('collaboration.disable') }}
        </BaseButton>
      </BaseGroupBox>
      <BaseGroupBox :title="t('collaboration.configuration')">
        <BaseAlert
          v-if="!status?.gateway.available"
          tone="warning"
          :title="t('collaboration.unavailable')"
        />
        <BaseAlert
          v-if="changed"
          tone="warning"
          :title="t('collaboration.changed')"
        />
        <form
          autocomplete="off"
          @submit.prevent="applyConfiguration"
        >
          <BaseCheckbox
            :model-value="draft?.enabled ?? false"
            :disabled="blocked || !status?.gateway.available"
            @update:model-value="setBoolean('enabled', $event)"
          >
            {{ t('collaboration.enabled') }}
          </BaseCheckbox>
          <BaseFormGrid>
            <BaseFormRow
              for="collaboration-bind"
              :label="t('collaboration.bind')"
            >
              <BaseTextField
                id="collaboration-bind"
                :model-value="draft?.bindAddress ?? ''"
                maxlength="256"
                :disabled="blocked"
                @update:model-value="setText('bindAddress', $event)"
              />
            </BaseFormRow>
            <BaseFormRow
              for="collaboration-origin"
              :label="t('collaboration.origin')"
            >
              <BaseTextField
                id="collaboration-origin"
                :model-value="draft?.advertisedOrigin ?? ''"
                maxlength="2048"
                :disabled="blocked"
                placeholder="https://pc-a.example:8788"
                @update:model-value="setText('advertisedOrigin', $event)"
              />
            </BaseFormRow>
          </BaseFormGrid>
          <BaseCheckbox
            :model-value="draft?.tlsProxy ?? false"
            :disabled="blocked"
            @update:model-value="setBoolean('tlsProxy', $event)"
          >
            {{ t('collaboration.tlsProxy') }}
          </BaseCheckbox>
          <p v-if="draft?.tlsProxy">
            {{ t('collaboration.proxyHint') }}
          </p>
          <BaseFormGrid v-else>
            <BaseFormRow
              for="collaboration-certificate"
              :label="t('collaboration.certificate')"
            >
              <BaseTextField
                id="collaboration-certificate"
                :model-value="draft?.tlsCertificate ?? ''"
                :disabled="blocked"
                maxlength="4096"
                @update:model-value="setText('tlsCertificate', $event)"
              />
            </BaseFormRow>
            <BaseFormRow
              for="collaboration-key"
              :label="t('collaboration.privateKey')"
              :hint="t('collaboration.pathsHint')"
            >
              <BaseTextField
                id="collaboration-key"
                :model-value="draft?.tlsPrivateKey ?? ''"
                :disabled="blocked"
                maxlength="4096"
                @update:model-value="setText('tlsPrivateKey', $event)"
              />
            </BaseFormRow>
          </BaseFormGrid>
          <label for="collaboration-origins">{{ t('collaboration.origins') }}</label><BaseTextArea
            id="collaboration-origins"
            v-model="allowedOrigins"
            :rows="3"
            maxlength="16384"
            :disabled="blocked"
          />
          <details>
            <summary>{{ t('collaboration.advanced') }}</summary><BaseFormGrid>
              <BaseFormRow
                v-for="limit in limits"
                :key="limit.key"
                :for="`collaboration-${limit.key}`"
                :label="t(`collaboration.limits.${limit.key}`)"
              >
                <input
                  :id="`collaboration-${limit.key}`"
                  :value="draft?.limits[limit.key] ?? 0"
                  type="number"
                  :min="limit.min"
                  :max="limit.max"
                  step="1"
                  :disabled="blocked"
                  @input="setLimit(limit.key, $event)"
                >
              </BaseFormRow>
            </BaseFormGrid>
          </details>
          <div class="collaboration-editor__actions">
            <BaseButton
              type="submit"
              variant="primary"
              :disabled="blocked || changed || !status?.gateway.available"
            >
              {{ t('collaboration.save') }}
            </BaseButton><BaseButton
              :disabled="working"
              @click="resetDraft"
            >
              {{ t('collaboration.reset') }}
            </BaseButton>
          </div>
        </form>
      </BaseGroupBox>
      <BaseGroupBox
        :title="t('collaboration.invites')"
        :description="t('collaboration.inviteHint')"
      >
        <GrantEditor
          v-model="grant"
          :disabled="blocked"
        />
        <p v-if="!canInvite">
          {{ t('collaboration.listenerRequired') }}
        </p>
        <BaseButton
          variant="primary"
          :disabled="!canInvite"
          @click="createInvitation"
        >
          {{ t('collaboration.create') }}
        </BaseButton>
        <section
          v-if="secret && joinLink"
          class="collaboration-editor__share"
        >
          <p>{{ t('collaboration.shareHint') }}</p><label for="collaboration-link">{{ t('collaboration.privateLink') }}</label>
          <BaseTextField
            id="collaboration-link"
            :model-value="joinLink"
            type="password"
            readonly
            revealable
            autocomplete="off"
          />
          <p>{{ t('collaboration.expires', { when: date(secret.expiresAt) }) }}</p>
          <div class="collaboration-editor__actions">
            <BaseButton @click="copyLink">
              {{ copied ? t('collaboration.copied') : t('collaboration.copy') }}
            </BaseButton><BaseButton @click="showQr = !showQr">
              {{ showQr ? t('collaboration.hideQr') : t('collaboration.qr') }}
            </BaseButton><BaseButton @click="secret = null">
              {{ t('collaboration.hide') }}
            </BaseButton>
          </div>
          <InvitationQr
            v-if="showQr"
            :value="joinLink"
          />
        </section>
      </BaseGroupBox>
      <BaseGroupBox :title="t(`collaboration.tabs.${tab}`)">
        <div
          class="collaboration-editor__actions"
          role="group"
          :aria-label="t('collaboration.title')"
        >
          <BaseButton
            v-for="name in tabs"
            :key="name"
            :aria-selected="tab === name"
            :pressed="tab === name"
            @click="owner.selectTab(name)"
          >
            {{ t(`collaboration.tabs.${name}`) }}
          </BaseButton>
        </div>
        <div class="collaboration-editor__actions">
          <BaseButton
            :disabled="loading"
            @click="owner.first"
          >
            {{ t('collaboration.refresh') }}
          </BaseButton><BaseButton
            :disabled="loading || !previous.length"
            @click="owner.previousPage"
          >
            {{ t('collaboration.previous') }}
          </BaseButton><BaseButton
            :disabled="loading || next === null"
            @click="owner.nextPage"
          >
            {{ t('collaboration.next') }}
          </BaseButton>
        </div><p>{{ t('collaboration.pageHint') }}</p>
        <p
          v-if="loading"
          role="status"
        >
          {{ t('collaboration.loading') }}
        </p>
        <template v-if="tab === 'requests'">
          <article
            v-for="request in pages.requests"
            :key="request.requestId"
            class="collaboration-editor__entry"
          >
            <h3>{{ request.clientLabel }} · {{ request.clientKind }}</h3><p>{{ t('collaboration.requestId') }}: <code>{{ request.requestId }}</code></p><p>{{ state(request.state) }} · {{ t('collaboration.expires', { when: date(request.expiresAt) }) }}</p><details><summary>{{ t('collaboration.grantSummary') }}</summary><pre>{{ JSON.stringify(request.grant, null, 2) }}</pre></details><div
              v-if="request.state === 'pending' && request.expiresAt > owner.now.value"
              class="collaboration-editor__actions"
            >
              <BaseButton
                :disabled="blocked"
                variant="primary"
                @click="confirmation = { action: 'approve', id: request.requestId, request: clone(request), grant: clone(request.grant) }"
              >
                {{ t('collaboration.approve') }}
              </BaseButton><BaseButton
                :disabled="blocked"
                @click="confirmation = { action: 'reject', id: request.requestId }"
              >
                {{ t('collaboration.reject') }}
              </BaseButton>
            </div>
          </article>
        </template>
        <template v-else-if="tab === 'principals'">
          <article
            v-for="person in pages.principals"
            :key="person.principalId"
            class="collaboration-editor__entry"
          >
            <h3>{{ person.clientLabel }} · {{ person.clientKind }}</h3><p>{{ t('collaboration.principalId') }}: <code>{{ person.principalId }}</code> · {{ t('collaboration.revision') }} {{ person.grantRevision }}</p><p v-if="person.revoked">
              {{ state('revoked') }}
            </p><details><summary>{{ t('collaboration.grantSummary') }}</summary><pre>{{ JSON.stringify(person.grant, null, 2) }}</pre></details><div
              v-if="!person.revoked"
              class="collaboration-editor__actions"
            >
              <BaseButton
                :disabled="blocked"
                @click="editPrincipal(person)"
              >
                {{ t('collaboration.editGrant') }}
              </BaseButton><BaseButton
                :disabled="blocked"
                variant="danger"
                @click="confirmation = { action: 'principal', id: person.principalId }"
              >
                {{ t('collaboration.revoke') }}
              </BaseButton>
            </div>
          </article>
        </template>
        <template v-else-if="tab === 'sessions'">
          <article
            v-for="session in pages.sessions"
            :key="session.sessionId"
            class="collaboration-editor__entry"
          >
            <h3>{{ session.clientLabel || session.principalId }}</h3><p>{{ t('collaboration.sessionId') }}: <code>{{ session.sessionId }}</code> · {{ session.clientKind }}</p><p>{{ t('collaboration.principalId') }}: <code>{{ session.principalId }}</code></p><p>{{ session.revoked ? state('revoked') : connections(session.sessionId) ? t('collaboration.online', { count: connections(session.sessionId) }) : t('collaboration.offline') }}</p><p>{{ t('collaboration.lastActivity') }}: {{ date(session.lastActivityAt) }} · {{ t('collaboration.expires', { when: date(session.expiresAt) }) }}</p><BaseButton
              v-if="!session.revoked"
              :disabled="blocked"
              @click="editPrincipal(session.principalId)"
            >
              {{ t('collaboration.editGrant') }}
            </BaseButton><BaseButton
              v-if="!session.revoked"
              :disabled="blocked"
              variant="danger"
              @click="confirmation = { action: 'session', id: session.sessionId }"
            >
              {{ t('collaboration.revoke') }}
            </BaseButton>
          </article>
        </template>
        <template v-else-if="tab === 'invitations'">
          <article
            v-for="invitation in pages.invitations"
            :key="invitation.inviteId"
            class="collaboration-editor__entry"
          >
            <code>{{ invitation.inviteId }}</code><p>{{ state(invitation.state) }} · {{ t('collaboration.expires', { when: date(invitation.expiresAt) }) }}</p><details><summary>{{ t('collaboration.grantSummary') }}</summary><pre>{{ JSON.stringify(invitation.grant, null, 2) }}</pre></details><BaseButton
              v-if="invitation.state === 'created'"
              :disabled="blocked"
              @click="confirmation = { action: 'invite', id: invitation.inviteId }"
            >
              {{ t('collaboration.revoke') }}
            </BaseButton>
          </article>
        </template>
        <template v-else>
          <article
            v-for="entry in pages.audit"
            :key="entry.cursor"
            class="collaboration-editor__entry"
          >
            <p>{{ date(entry.timestamp) }} · {{ entry.action }} · {{ entry.outcome }}</p><code>{{ entry.principalId }} · {{ entry.resourceId }}</code><details v-if="entry.details">
              <summary>{{ t('collaboration.details') }}</summary><pre>{{ JSON.stringify(entry.details, null, 2) }}</pre>
            </details>
          </article>
        </template>
        <p v-if="!loading && !pages[tab].length">
          {{ t('collaboration.empty') }}
        </p>
      </BaseGroupBox>
      <BaseGroupBox
        v-if="principalDraft"
        :title="t('collaboration.editGrant')"
      >
        <code>{{ principalDraft.id }}</code><p>{{ t('collaboration.revision') }} {{ principalDraft.revision }}</p><GrantEditor
          :key="`${principalDraft.id}:${principalDraft.revision}`"
          v-model="principalDraft.grant"
          :disabled="blocked"
        /><div class="collaboration-editor__actions">
          <BaseButton
            :disabled="blocked"
            variant="primary"
            @click="confirmGrant"
          >
            {{ t('collaboration.grantSave') }}
          </BaseButton><BaseButton @click="closePrincipal">
            {{ t('common.cancel') }}
          </BaseButton>
        </div>
      </BaseGroupBox>
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
      <code v-if="confirmation?.id">{{ confirmation.id }}</code><p v-if="confirmation?.request">
        {{ confirmation.request.clientLabel }} · {{ confirmation.request.clientKind }}
      </p><pre v-if="confirmation?.grant">{{ JSON.stringify(confirmation.grant, null, 2) }}</pre>
    </BaseConfirmDialog>
  </div>
</template>

<style scoped>
.collaboration-editor, .collaboration-editor form { display: flex; flex-direction: column; gap: var(--space-4); }
.collaboration-editor__actions { display: flex; flex-wrap: wrap; gap: var(--space-2); margin: var(--space-3) 0; }
.collaboration-editor__entry { margin: var(--space-4) 0; padding: var(--space-4) 0; border-top: 1px solid var(--color-border-subtle); }
.collaboration-editor__share { padding: var(--space-4); border: 1px solid var(--color-border-focus); border-radius: var(--radius-md); }
.collaboration-editor__fingerprint { display: block; user-select: all; }
code { overflow-wrap: anywhere; } pre { max-height: 300px; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; } summary { cursor: pointer; }
input[type=number] { width: 100%; padding: var(--space-3); background: var(--color-bg-field); color: var(--color-text); border: 1px solid var(--color-border); }
</style>
