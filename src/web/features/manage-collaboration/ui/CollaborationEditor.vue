<script setup lang="ts">
import SettingsDisclosure from '@/shared/ui/settings-disclosure/SettingsDisclosure.vue';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { createCollaborationOwner } from '@/entities/collaboration/model/owner.ts';
import { viewerGrant, type AdminTab, type Configuration, type Grant, type PairingRequest, type Principal, type RemoteLimits } from '@/entities/collaboration/api/types.ts';
import { parseGrant, prepareConfiguration, splitIds } from '@/entities/collaboration/lib/grants.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import type { StatusTone } from '@/shared/ui/status/status-types.ts';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseTextArea from '@/shared/ui/text-area/BaseTextArea.vue';
import GrantEditor from './GrantEditor.vue';
import InvitationQr from './InvitationQr.vue';

const { t, te } = useI18n();
const area = ref<'overview' | 'people' | 'network'>('overview');
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
const listener = computed<{ label: string; tone: StatusTone }>(() => {
  if (!status.value) return { label: '', tone: 'neutral' };
  if (!status.value.gateway.running) return { label: t('collaboration.stopped'), tone: 'neutral' };
  return status.value.configuration.enabled
    ? { label: t('collaboration.running'), tone: 'success' }
    : { label: t('collaboration.applying'), tone: 'warning' };
});
const stateTone = (value: string): StatusTone => {
  if (value === 'pending') return 'warning';
  if (['approved', 'created', 'redeemed'].includes(value)) return 'success';
  if (['rejected', 'revoked', 'expired'].includes(value)) return 'error';
  return 'neutral';
};
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
      <nav
        class="collaboration-editor__areas"
        :aria-label="t('collaboration.title')"
      >
        <BaseButton
          v-for="item in (['overview', 'people', 'network'] as const)"
          :key="item"
          :pressed="area === item"
          @click="area = item"
        >
          {{ t(`settingsLayout.${item}`) }}
        </BaseButton>
      </nav>
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
      <BaseGroupBox
        v-show="area === 'overview'"
        :title="t('settingsLayout.overview')"
        :description="t('collaboration.description')"
      >
        <dl class="collaboration-editor__metrics">
          <div>
            <dt>{{ t('collaboration.metrics.connections') }}</dt>
            <dd>{{ status.gateway.connections }}</dd>
          </div>
          <div>
            <dt>{{ t('collaboration.metrics.sessions') }}</dt>
            <dd>{{ status.activeSessions }}</dd>
          </div>
          <div>
            <dt>{{ t('collaboration.metrics.pending') }}</dt>
            <dd>{{ status.pendingRequests }}</dd>
          </div>
          <div>
            <dt>{{ t('collaboration.metrics.listener') }}</dt>
            <dd>
              <BaseStatus
                :label="listener.label"
                :tone="listener.tone"
              />
            </dd>
          </div>
        </dl>
        <p
          v-if="checkedAt"
          class="collaboration-editor__muted"
        >
          {{ t('collaboration.checked', { when: date(checkedAt) }) }}
        </p>
        <BaseAlert
          v-if="status.configuration.enabled && status.gateway.configurationRevision !== status.configurationRevision"
          tone="info"
          :title="t('collaboration.applying')"
        />
        <BaseAlert
          v-if="status.gateway.lastError"
          :title="status.gateway.lastError"
        />
        <p
          v-if="status.gateway.running && status.gateway.certificateFingerprint"
          class="collaboration-editor__muted"
        >
          {{ t('collaboration.fingerprint') }}:
          <code class="collaboration-editor__fingerprint">{{ status.gateway.certificateFingerprint }}</code>
        </p>
        <p
          v-else-if="status.configuration.tlsProxy"
          class="collaboration-editor__muted"
        >
          {{ t('collaboration.proxyFingerprint') }}
        </p>
        <div
          v-if="status.configuration.enabled"
          class="collaboration-editor__actions"
        >
          <BaseButton
            variant="danger"
            :disabled="blocked"
            @click="requestDisable"
          >
            {{ t('collaboration.disable') }}
          </BaseButton>
        </div>
        <SettingsDisclosure
          class="collaboration-editor__technical"
          :title="t('settingsLayout.technical')"
        >
          <p>{{ t('collaboration.host') }}: <code>{{ status.hostId }}</code></p>
          <p>{{ t('collaboration.instance') }}: <code>{{ status.hostInstanceId }}</code></p>
        </SettingsDisclosure>
      </BaseGroupBox>
      <BaseGroupBox
        v-show="area === 'network'"
        :title="t('collaboration.configuration')"
      >
        <form
          autocomplete="off"
          @submit.prevent="applyConfiguration"
        >
          <section class="collaboration-editor__section">
            <BaseSwitch
              :model-value="draft?.enabled ?? false"
              :disabled="blocked || !status?.gateway.available"
              @update:model-value="setBoolean('enabled', $event)"
            >
              {{ t('collaboration.enabled') }}
            </BaseSwitch>
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
          </section>
          <section class="collaboration-editor__section">
            <h3>{{ t('collaboration.tlsTitle') }}</h3>
            <BaseSwitch
              :model-value="draft?.tlsProxy ?? false"
              :disabled="blocked"
              @update:model-value="setBoolean('tlsProxy', $event)"
            >
              {{ t('collaboration.tlsProxy') }}
            </BaseSwitch>
            <p
              v-if="draft?.tlsProxy"
              class="collaboration-editor__muted"
            >
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
          </section>
          <section class="collaboration-editor__section">
            <label for="collaboration-origins">{{ t('collaboration.originsTitle') }}</label>
            <BaseTextArea
              id="collaboration-origins"
              v-model="allowedOrigins"
              :rows="3"
              maxlength="16384"
              :disabled="blocked"
            />
          </section>
          <SettingsDisclosure :title="t('collaboration.advanced')">
            <BaseFormGrid>
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
          </SettingsDisclosure>
          <div class="collaboration-editor__actions">
            <BaseButton
              type="submit"
              variant="primary"
              :disabled="blocked || changed || !status?.gateway.available"
            >
              {{ t('collaboration.save') }}
            </BaseButton>
            <BaseButton
              :disabled="working"
              @click="resetDraft"
            >
              {{ t('collaboration.reset') }}
            </BaseButton>
          </div>
        </form>
      </BaseGroupBox>
      <BaseGroupBox
        v-show="area === 'people'"
        :title="t('settingsLayout.people')"
      >
        <div
          class="collaboration-editor__toolbar"
        >
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
            </BaseButton>
            <BaseButton
              :disabled="loading || !previous.length"
              @click="owner.previousPage"
            >
              {{ t('collaboration.previous') }}
            </BaseButton>
            <BaseButton
              :disabled="loading || next === null"
              @click="owner.nextPage"
            >
              {{ t('collaboration.next') }}
            </BaseButton>
          </div>
        </div>
        <p class="collaboration-editor__muted">
          {{ t('collaboration.pageHint') }}
        </p>
        <p
          v-if="loading"
          role="status"
          class="collaboration-editor__muted"
        >
          {{ t('collaboration.loading') }}
        </p>
        <div
          v-if="tab === 'requests'"
          class="collaboration-editor__list"
        >
          <article
            v-for="request in pages.requests"
            :key="request.requestId"
            class="collaboration-editor__card"
          >
            <header class="collaboration-editor__card-head">
              <h3>{{ request.clientLabel }} · {{ request.clientKind }}</h3>
              <BaseStatus
                :label="state(request.state)"
                :tone="stateTone(request.state)"
              />
            </header>
            <p class="collaboration-editor__meta">
              {{ t('collaboration.requestId') }}: <code>{{ request.requestId }}</code> · {{ t('collaboration.expires', { when: date(request.expiresAt) }) }}
            </p>
            <SettingsDisclosure :title="t('collaboration.grantSummary')">
              <pre>{{ JSON.stringify(request.grant, null, 2) }}</pre>
            </SettingsDisclosure>
            <div
              v-if="request.state === 'pending' && request.expiresAt > owner.now.value"
              class="collaboration-editor__actions"
            >
              <BaseButton
                :disabled="blocked"
                variant="primary"
                @click="confirmation = { action: 'approve', id: request.requestId, request: clone(request), grant: clone(request.grant) }"
              >
                {{ t('collaboration.approve') }}
              </BaseButton>
              <BaseButton
                :disabled="blocked"
                @click="confirmation = { action: 'reject', id: request.requestId }"
              >
                {{ t('collaboration.reject') }}
              </BaseButton>
            </div>
          </article>
        </div>
        <div
          v-else-if="tab === 'principals'"
          class="collaboration-editor__list"
        >
          <article
            v-for="person in pages.principals"
            :key="person.principalId"
            class="collaboration-editor__card"
          >
            <header class="collaboration-editor__card-head">
              <h3>{{ person.clientLabel }} · {{ person.clientKind }}</h3>
              <BaseStatus
                v-if="person.revoked"
                :label="state('revoked')"
                tone="error"
              />
            </header>
            <p class="collaboration-editor__meta">
              {{ t('collaboration.principalId') }}: <code>{{ person.principalId }}</code> · {{ t('collaboration.revision') }} {{ person.grantRevision }}
            </p>
            <SettingsDisclosure :title="t('collaboration.grantSummary')">
              <pre>{{ JSON.stringify(person.grant, null, 2) }}</pre>
            </SettingsDisclosure>
            <div
              v-if="!person.revoked"
              class="collaboration-editor__actions"
            >
              <BaseButton
                :disabled="blocked"
                @click="editPrincipal(person)"
              >
                {{ t('collaboration.editGrant') }}
              </BaseButton>
              <BaseButton
                :disabled="blocked"
                variant="danger"
                @click="confirmation = { action: 'principal', id: person.principalId }"
              >
                {{ t('collaboration.revoke') }}
              </BaseButton>
            </div>
          </article>
        </div>
        <div
          v-else-if="tab === 'sessions'"
          class="collaboration-editor__list"
        >
          <article
            v-for="session in pages.sessions"
            :key="session.sessionId"
            class="collaboration-editor__card"
          >
            <header class="collaboration-editor__card-head">
              <h3>{{ session.clientLabel || session.principalId }}</h3>
              <BaseStatus
                :label="session.revoked ? state('revoked') : connections(session.sessionId) ? t('collaboration.online', { count: connections(session.sessionId) }) : t('collaboration.offline')"
                :tone="session.revoked ? 'error' : connections(session.sessionId) ? 'success' : 'neutral'"
              />
            </header>
            <p class="collaboration-editor__meta">
              {{ t('collaboration.sessionId') }}: <code>{{ session.sessionId }}</code> · {{ session.clientKind }}
            </p>
            <p class="collaboration-editor__meta">
              {{ t('collaboration.principalId') }}: <code>{{ session.principalId }}</code>
            </p>
            <p class="collaboration-editor__meta">
              {{ t('collaboration.lastActivity') }}: {{ date(session.lastActivityAt) }} · {{ t('collaboration.expires', { when: date(session.expiresAt) }) }}
            </p>
            <div
              v-if="!session.revoked"
              class="collaboration-editor__actions"
            >
              <BaseButton
                :disabled="blocked"
                @click="editPrincipal(session.principalId)"
              >
                {{ t('collaboration.editGrant') }}
              </BaseButton>
              <BaseButton
                :disabled="blocked"
                variant="danger"
                @click="confirmation = { action: 'session', id: session.sessionId }"
              >
                {{ t('collaboration.revoke') }}
              </BaseButton>
            </div>
          </article>
        </div>
        <div
          v-else-if="tab === 'invitations'"
          class="collaboration-editor__list"
        >
          <article
            v-for="invitation in pages.invitations"
            :key="invitation.inviteId"
            class="collaboration-editor__card"
          >
            <header class="collaboration-editor__card-head">
              <code>{{ invitation.inviteId }}</code>
              <BaseStatus
                :label="state(invitation.state)"
                :tone="stateTone(invitation.state)"
              />
            </header>
            <p class="collaboration-editor__meta">
              {{ t('collaboration.expires', { when: date(invitation.expiresAt) }) }}
            </p>
            <SettingsDisclosure :title="t('collaboration.grantSummary')">
              <pre>{{ JSON.stringify(invitation.grant, null, 2) }}</pre>
            </SettingsDisclosure>
            <div
              v-if="invitation.state === 'created'"
              class="collaboration-editor__actions"
            >
              <BaseButton
                :disabled="blocked"
                variant="danger"
                @click="confirmation = { action: 'invite', id: invitation.inviteId }"
              >
                {{ t('collaboration.revoke') }}
              </BaseButton>
            </div>
          </article>
        </div>
        <div
          v-else
          class="collaboration-editor__list"
        >
          <article
            v-for="entry in pages.audit"
            :key="entry.cursor"
            class="collaboration-editor__card"
          >
            <p class="collaboration-editor__meta">
              {{ date(entry.timestamp) }} · {{ entry.action }} · {{ entry.outcome }}
            </p>
            <code class="collaboration-editor__meta">{{ entry.principalId }} · {{ entry.resourceId }}</code>
            <SettingsDisclosure
              v-if="entry.details"
              :title="t('collaboration.details')"
            >
              <pre>{{ JSON.stringify(entry.details, null, 2) }}</pre>
            </SettingsDisclosure>
          </article>
        </div>
        <p
          v-if="!loading && !pages[tab].length"
          class="collaboration-editor__muted"
        >
          {{ t('collaboration.empty') }}
        </p>
      </BaseGroupBox>
      <BaseGroupBox
        v-show="area === 'people'"
        :title="t('collaboration.invites')"
        :description="t('collaboration.inviteHint')"
      >
        <SettingsDisclosure
          class="collaboration-editor__permissions"
          initially-open
          :title="t('collaboration.grantSummary')"
        >
          <GrantEditor
            v-model="grant"
            :disabled="blocked"
          />
        </SettingsDisclosure>
        <p
          v-if="!canInvite"
          class="collaboration-editor__muted"
        >
          {{ t('collaboration.listenerRequired') }}
        </p>
        <div class="collaboration-editor__actions">
          <BaseButton
            variant="primary"
            :disabled="!canInvite"
            @click="createInvitation"
          >
            {{ t('collaboration.create') }}
          </BaseButton>
        </div>
        <section
          v-if="secret && joinLink"
          class="collaboration-editor__share"
        >
          <p class="collaboration-editor__muted">
            {{ t('collaboration.shareHint') }}
          </p>
          <label for="collaboration-link">{{ t('collaboration.privateLink') }}</label>
          <BaseTextField
            id="collaboration-link"
            :model-value="joinLink"
            type="password"
            readonly
            revealable
            autocomplete="off"
          />
          <p class="collaboration-editor__muted">
            {{ t('collaboration.expires', { when: date(secret.expiresAt) }) }}
          </p>
          <div class="collaboration-editor__actions">
            <BaseButton @click="copyLink">
              {{ copied ? t('collaboration.copied') : t('collaboration.copy') }}
            </BaseButton>
            <BaseButton @click="showQr = !showQr">
              {{ showQr ? t('collaboration.hideQr') : t('collaboration.qr') }}
            </BaseButton>
            <BaseButton @click="secret = null">
              {{ t('collaboration.hide') }}
            </BaseButton>
          </div>
          <InvitationQr
            v-if="showQr"
            :value="joinLink"
          />
        </section>
      </BaseGroupBox>
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
      <pre v-if="confirmation?.grant">{{ JSON.stringify(confirmation.grant, null, 2) }}</pre>
    </BaseConfirmDialog>
  </div>
</template>

<style scoped>
.collaboration-editor, .collaboration-editor form { display: flex; flex-direction: column; gap: var(--space-4); }
.collaboration-editor__areas { display: flex; flex-wrap: wrap; gap: var(--space-2); }
.collaboration-editor__pending { align-self: flex-start; }
.collaboration-editor__actions { display: flex; flex-wrap: wrap; gap: var(--space-2); margin: var(--space-2) 0 0; }
.collaboration-editor__toolbar { display: flex; flex-wrap: wrap; justify-content: space-between; gap: var(--space-3); }
.collaboration-editor__section { display: flex; flex-direction: column; gap: var(--space-3); padding-top: var(--space-4); border-top: 1px solid var(--color-border-subtle); }
.collaboration-editor__section > h3 { margin: 0; font-size: var(--font-size-sm); color: var(--color-text-muted); text-transform: uppercase; letter-spacing: .04em; }
.collaboration-editor__metrics { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: var(--space-3); margin: 0; }
.collaboration-editor__metrics > div { display: flex; flex-direction: column; gap: var(--space-2); padding: var(--space-4); background: var(--color-bg-field); border-radius: var(--radius-md); }
.collaboration-editor__metrics dt { color: var(--color-text-muted); font-size: var(--font-size-sm); }
.collaboration-editor__metrics dd { margin: 0; font-size: 20px; font-weight: var(--font-weight-semibold); }
.collaboration-editor__list { display: flex; flex-direction: column; gap: var(--space-3); margin-top: var(--space-3); }
.collaboration-editor__card { display: flex; flex-direction: column; gap: var(--space-2); padding: var(--space-4); background: var(--color-bg-field); border: 1px solid var(--color-border-subtle); border-radius: var(--radius-md); min-width: 0; }
.collaboration-editor__card-head { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: var(--space-2); }
.collaboration-editor__card h3 { margin: 0; font-size: var(--font-size-md, 15px); overflow-wrap: anywhere; }
.collaboration-editor__meta, .collaboration-editor__muted { margin: 0; color: var(--color-text-muted); font-size: var(--font-size-sm); line-height: var(--line-height-normal); }
.collaboration-editor__share { display: flex; flex-direction: column; gap: var(--space-3); padding: var(--space-4); border: 1px solid var(--color-border-focus); border-radius: var(--radius-md); }
.collaboration-editor__fingerprint { display: block; user-select: all; }
code { overflow-wrap: anywhere; } pre { max-height: 300px; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; }
input[type=number] { width: 100%; padding: var(--space-3); background: var(--color-bg-field); color: var(--color-text); border: 1px solid var(--color-border); }
.collaboration-editor__technical { order: 1; color: var(--color-text-muted); }
.collaboration-editor__technical code { overflow-wrap: anywhere; }
.collaboration-editor > :deep(.base-group-box) { border: 0; border-top: var(--border-width) solid var(--color-border); background: transparent; padding: var(--space-6) 0; }
</style>
