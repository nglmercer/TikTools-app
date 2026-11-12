<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { computed } from 'vue';
import type { AdminTab, PageRows } from '@/entities/collaboration/api/types.ts';
import type { CollaborationConfirmation } from '../model/confirmation.ts';
import { useCollaborationPresentation } from '../lib/use-collaboration-presentation.ts';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import SettingsDisclosure from '@/shared/ui/settings-disclosure/SettingsDisclosure.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';
const props = defineProps<{ pages: PageRows; loading: boolean; blocked: boolean; hasPrevious: boolean; hasNext: boolean; now: number; sessionConnections: Record<string, number> }>();
const tab = defineModel<AdminTab>('tab', { required: true });
const emit = defineEmits<{ refresh: []; previous: []; next: []; edit: [id: string]; confirm: [confirmation: CollaborationConfirmation] }>();
const { t } = useI18n();
const tabs: AdminTab[] = ['requests', 'principals', 'sessions', 'invitations', 'audit'];
const adminTabs = computed(() => tabs.map(value => ({ value, label: t(`collaboration.tabs.${value}`) })));
const { date, state, stateTone } = useCollaborationPresentation();
const connections = (id: string): number => props.sessionConnections[id] ?? 0;
</script>

<template>
  <BaseGroupBox
    :title="t('settingsLayout.people')"
  >
    <BaseTabs
      v-model="tab"
      :tabs="adminTabs"
      :label="t('settingsLayout.people')"
      class="collaboration-editor__admin-tabs"
    >
      <div
        class="collaboration-editor__toolbar"
      >
        <div class="collaboration-editor__actions">
          <BaseButton
            :disabled="loading"
            @click="emit('refresh')"
          >
            {{ t('collaboration.refresh') }}
          </BaseButton>
          <BaseButton
            :disabled="loading || !hasPrevious"
            @click="emit('previous')"
          >
            {{ t('collaboration.previous') }}
          </BaseButton>
          <BaseButton
            :disabled="loading || !hasNext"
            @click="emit('next')"
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
            <BaseJsonView
              :value="request.grant"
              max-height="300px"
            />
          </SettingsDisclosure>
          <div
            v-if="request.state === 'pending' && request.expiresAt > now"
            class="collaboration-editor__actions"
          >
            <BaseButton
              :disabled="blocked"
              variant="primary"
              @click="emit('confirm', { action: 'approve', id: request.requestId, request: request, grant: request.grant })"
            >
              {{ t('collaboration.approve') }}
            </BaseButton>
            <BaseButton
              :disabled="blocked"
              @click="emit('confirm', { action: 'reject', id: request.requestId })"
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
            <BaseJsonView
              :value="person.grant"
              max-height="300px"
            />
          </SettingsDisclosure>
          <div
            v-if="!person.revoked"
            class="collaboration-editor__actions"
          >
            <BaseButton
              :disabled="blocked"
              @click="emit('edit', person.principalId)"
            >
              {{ t('collaboration.editGrant') }}
            </BaseButton>
            <BaseButton
              :disabled="blocked"
              variant="danger"
              @click="emit('confirm', { action: 'principal', id: person.principalId })"
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
              @click="emit('edit', session.principalId)"
            >
              {{ t('collaboration.editGrant') }}
            </BaseButton>
            <BaseButton
              :disabled="blocked"
              variant="danger"
              @click="emit('confirm', { action: 'session', id: session.sessionId })"
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
            <BaseJsonView
              :value="invitation.grant"
              max-height="300px"
            />
          </SettingsDisclosure>
          <div
            v-if="invitation.state === 'created'"
            class="collaboration-editor__actions"
          >
            <BaseButton
              :disabled="blocked"
              variant="danger"
              @click="emit('confirm', { action: 'invite', id: invitation.inviteId })"
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
            <BaseJsonView
              :value="entry.details"
              max-height="300px"
            />
          </SettingsDisclosure>
        </article>
      </div>
      <p
        v-if="!loading && !pages[tab].length"
        class="collaboration-editor__muted"
      >
        {{ t('collaboration.empty') }}
      </p>
    </BaseTabs>
  </BaseGroupBox>
</template>
