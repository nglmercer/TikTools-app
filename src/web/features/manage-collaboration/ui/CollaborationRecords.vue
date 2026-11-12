<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { computed } from 'vue';
import type { AdminTab, PageRows } from '@/entities/collaboration/api/types.ts';
import type { CollaborationConfirmation } from '../model/confirmation.ts';
import { useCollaborationPresentation } from '../lib/use-collaboration-presentation.ts';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import SettingsDisclosure from '@/shared/ui/settings-disclosure/SettingsDisclosure.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';
import CollabRecord from './CollabRecord.vue';
import CollabSection from './CollabSection.vue';
import GrantSummary from './GrantSummary.vue';
const props = defineProps<{ pages: PageRows; loading: boolean; blocked: boolean; hasPrevious: boolean; hasNext: boolean; now: number; sessionConnections: Record<string, number> }>();
const tab = defineModel<AdminTab>('tab', { required: true });
const emit = defineEmits<{ refresh: []; previous: []; next: []; edit: [id: string]; confirm: [confirmation: CollaborationConfirmation] }>();
const { t } = useI18n();
const tabs: AdminTab[] = ['requests', 'principals', 'sessions', 'invitations', 'audit'];
// The count only reflects the loaded page; it flags where something needs attention without implying a total.
const adminTabs = computed(() => tabs.map(value => ({ value, label: t(`collaboration.tabs.${value}`), badge: value === 'requests' ? pendingCount.value || undefined : undefined })));
const pendingCount = computed(() => props.pages.requests.filter(request => request.state === 'pending' && request.expiresAt > props.now).length);
const { date, state, stateTone } = useCollaborationPresentation();
const connections = (id: string): number => props.sessionConnections[id] ?? 0;
const initial = (label: string): string => (label.trim()[0] ?? '?').toUpperCase();
const outcomeTone = (outcome: string) => /succe|ok|approved|allowed/i.test(outcome) ? 'success' as const : /fail|den|reject|error|conflict/i.test(outcome) ? 'error' as const : 'neutral' as const;
</script>

<template>
  <CollabSection
    :title="t('settingsLayout.people')"
  >
    <BaseTabs
      v-model="tab"
      :tabs="adminTabs"
      :label="t('settingsLayout.people')"
      class="collaboration-editor__admin-tabs"
    >
      <div class="collaboration-editor__toolbar">
        <p class="collaboration-editor__muted collaboration-editor__toolbar-hint">
          {{ t('collaboration.pageHint') }}
        </p>
        <div class="collaboration-editor__actions">
          <BaseButton
            size="sm"
            :disabled="loading"
            @click="emit('refresh')"
          >
            {{ t('collaboration.refresh') }}
          </BaseButton>
          <BaseButton
            size="sm"
            :disabled="loading || !hasPrevious"
            @click="emit('previous')"
          >
            {{ t('collaboration.previous') }}
          </BaseButton>
          <BaseButton
            size="sm"
            :disabled="loading || !hasNext"
            @click="emit('next')"
          >
            {{ t('collaboration.next') }}
          </BaseButton>
        </div>
      </div>
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
        <CollabRecord
          v-for="request in pages.requests"
          :key="request.requestId"
          :initial="initial(request.clientLabel)"
        >
          <template #title>
            {{ request.clientLabel }} <span class="collab-record__kind">{{ request.clientKind }}</span>
          </template>
          <template #status>
            <BaseStatus
              :label="state(request.state)"
              :tone="stateTone(request.state)"
            />
          </template>
          <template #meta>
            <span>{{ t('collaboration.requestId') }}: <code class="collab-id">{{ request.requestId }}</code></span>
            <span>{{ t('collaboration.expires', { when: date(request.expiresAt) }) }}</span>
          </template>
          <GrantSummary :grant="request.grant" />
          <SettingsDisclosure :title="t('collaboration.grantSummary')">
            <BaseJsonView
              :value="request.grant"
              max-height="300px"
            />
          </SettingsDisclosure>
          <template
            v-if="request.state === 'pending' && request.expiresAt > now"
            #actions
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
          </template>
        </CollabRecord>
      </div>
      <div
        v-else-if="tab === 'principals'"
        class="collaboration-editor__list"
      >
        <CollabRecord
          v-for="person in pages.principals"
          :key="person.principalId"
          :initial="initial(person.clientLabel)"
        >
          <template #title>
            {{ person.clientLabel }} <span class="collab-record__kind">{{ person.clientKind }}</span>
          </template>
          <template
            v-if="person.revoked"
            #status
          >
            <BaseStatus
              :label="state('revoked')"
              tone="error"
            />
          </template>
          <template #meta>
            <span>{{ t('collaboration.principalId') }}: <code class="collab-id">{{ person.principalId }}</code></span>
            <span>{{ t('collaboration.revision') }} {{ person.grantRevision }}</span>
          </template>
          <GrantSummary :grant="person.grant" />
          <SettingsDisclosure :title="t('collaboration.grantSummary')">
            <BaseJsonView
              :value="person.grant"
              max-height="300px"
            />
          </SettingsDisclosure>
          <template
            v-if="!person.revoked"
            #actions
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
          </template>
        </CollabRecord>
      </div>
      <div
        v-else-if="tab === 'sessions'"
        class="collaboration-editor__list"
      >
        <CollabRecord
          v-for="session in pages.sessions"
          :key="session.sessionId"
          :initial="initial(session.clientLabel || session.clientKind)"
        >
          <template #title>
            {{ session.clientLabel || session.principalId }} <span class="collab-record__kind">{{ session.clientKind }}</span>
          </template>
          <template #status>
            <BaseStatus
              :label="session.revoked ? state('revoked') : connections(session.sessionId) ? t('collaboration.online', { count: connections(session.sessionId) }) : t('collaboration.offline')"
              :tone="session.revoked ? 'error' : connections(session.sessionId) ? 'success' : 'neutral'"
            />
          </template>
          <template #meta>
            <span>{{ t('collaboration.sessionId') }}: <code class="collab-id">{{ session.sessionId }}</code></span>
            <span>{{ t('collaboration.principalId') }}: <code class="collab-id">{{ session.principalId }}</code></span>
            <span>{{ t('collaboration.lastActivity') }}: {{ date(session.lastActivityAt) }}</span>
            <span>{{ t('collaboration.expires', { when: date(session.expiresAt) }) }}</span>
          </template>
          <template
            v-if="!session.revoked"
            #actions
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
          </template>
        </CollabRecord>
      </div>
      <div
        v-else-if="tab === 'invitations'"
        class="collaboration-editor__list"
      >
        <CollabRecord
          v-for="invitation in pages.invitations"
          :key="invitation.inviteId"
        >
          <template #title>
            <code class="collab-id">{{ invitation.inviteId }}</code>
          </template>
          <template #status>
            <BaseStatus
              :label="state(invitation.state)"
              :tone="stateTone(invitation.state)"
            />
          </template>
          <template #meta>
            <span>{{ t('collaboration.expires', { when: date(invitation.expiresAt) }) }}</span>
          </template>
          <GrantSummary :grant="invitation.grant" />
          <SettingsDisclosure :title="t('collaboration.grantSummary')">
            <BaseJsonView
              :value="invitation.grant"
              max-height="300px"
            />
          </SettingsDisclosure>
          <template
            v-if="invitation.state === 'created'"
            #actions
          >
            <BaseButton
              :disabled="blocked"
              variant="danger"
              @click="emit('confirm', { action: 'invite', id: invitation.inviteId })"
            >
              {{ t('collaboration.revoke') }}
            </BaseButton>
          </template>
        </CollabRecord>
      </div>
      <div
        v-else
        class="collaboration-editor__list collaboration-editor__list--log"
      >
        <CollabRecord
          v-for="entry in pages.audit"
          :key="entry.cursor"
        >
          <template #title>
            <code class="collab-action">{{ entry.action }}</code>
          </template>
          <template #status>
            <BaseStatus
              :label="entry.outcome"
              :tone="outcomeTone(entry.outcome)"
            />
          </template>
          <template #meta>
            <span>{{ date(entry.timestamp) }}</span>
            <code class="collab-id">{{ entry.principalId }} · {{ entry.resourceId }}</code>
          </template>
          <SettingsDisclosure
            v-if="entry.details"
            :title="t('collaboration.details')"
          >
            <BaseJsonView
              :value="entry.details"
              max-height="300px"
            />
          </SettingsDisclosure>
        </CollabRecord>
      </div>
      <BaseEmptyState
        v-if="!loading && !pages[tab].length"
        :title="t('collaboration.empty')"
      />
    </BaseTabs>
  </CollabSection>
</template>
