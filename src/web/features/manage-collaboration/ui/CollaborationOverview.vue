<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { computed } from 'vue';
import type { CollaborationStatus } from '@/entities/collaboration/api/types.ts';
import { useCollaborationPresentation } from '../lib/use-collaboration-presentation.ts';
import SettingsDisclosure from '@/shared/ui/settings-disclosure/SettingsDisclosure.vue';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import CollabSection from './CollabSection.vue';
const props = defineProps<{ status: CollaborationStatus; checkedAt: number | null; blocked: boolean }>();
const emit = defineEmits<{ disable: []; navigate: [area: 'people' | 'invite' | 'direct'] }>();
const { t } = useI18n();
const { date, listenerStatus } = useCollaborationPresentation();
const listener = computed(() => listenerStatus(props.status));
// One next step, chosen from the current state, instead of leaving the owner to guess which tab matters.
const next = computed<{ message: string; action: string; area: 'people' | 'invite' | 'direct' } | null>(() => {
  if (!props.status.gateway.available) return null;
  if (props.status.pendingRequests > 0) return { message: t('collaboration.heroPending', { count: props.status.pendingRequests }), action: t('settingsLayout.requests'), area: 'people' };
  if (!props.status.gateway.running || !props.status.configuration.enabled) return { message: t('collaboration.heroSetup'), action: t('collaboration.openDirect'), area: 'direct' };
  return { message: t('collaboration.heroReady'), action: t('collaboration.invites'), area: 'invite' };
});
</script>

<template>
  <div class="overview">
    <section
      class="overview__hero"
      :class="`overview__hero--${listener.tone}`"
    >
      <div class="overview__state">
        <BaseStatus
          class="overview__headline"
          :label="listener.label"
          :tone="listener.tone"
        />
        <p
          v-if="status.gateway.running && status.gateway.address"
          class="collaboration-editor__muted"
        >
          {{ t('collaboration.listenerAddress', { address: status.gateway.address }) }}
        </p>
        <p
          v-if="next"
          class="overview__next"
        >
          {{ next.message }}
        </p>
        <p
          v-else
          class="overview__next overview__next--warning"
        >
          {{ t('collaboration.unavailable') }}
        </p>
      </div>
      <BaseButton
        v-if="next"
        class="overview__cta"
        variant="primary"
        @click="emit('navigate', next.area)"
      >
        {{ next.action }}
      </BaseButton>
    </section>
    <BaseAlert
      v-if="status.configuration.enabled && status.gateway.configurationRevision !== status.configurationRevision"
      tone="info"
      :title="t('collaboration.applying')"
    />
    <BaseAlert
      v-if="status.gateway.lastError"
      :title="status.gateway.lastError"
    />
    <dl class="collaboration-editor__metrics">
      <div>
        <dt>{{ t('collaboration.metrics.connections') }}</dt>
        <dd>{{ status.gateway.connections }}</dd>
      </div>
      <div>
        <dt>{{ t('collaboration.metrics.sessions') }}</dt>
        <dd>{{ status.activeSessions }}</dd>
      </div>
      <div :class="{ 'collaboration-editor__metric--attention': status.pendingRequests > 0 }">
        <dt>{{ t('collaboration.metrics.pending') }}</dt>
        <dd>{{ status.pendingRequests }}</dd>
      </div>
    </dl>
    <p
      v-if="checkedAt"
      class="collaboration-editor__muted"
    >
      {{ t('collaboration.checked', { when: date(checkedAt) }) }}
    </p>
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
    <SettingsDisclosure
      class="collaboration-editor__technical"
      :title="t('settingsLayout.technical')"
    >
      <p>{{ t('collaboration.host') }}: <code>{{ status.hostId }}</code></p>
      <p>{{ t('collaboration.instance') }}: <code>{{ status.hostInstanceId }}</code></p>
    </SettingsDisclosure>
    <CollabSection
      v-if="status.configuration.enabled"
      class="collab-section--danger"
      :title="t('collaboration.dangerTitle')"
      :description="t('collaboration.disableMessage')"
    >
      <template #actions>
        <BaseButton
          variant="danger"
          :disabled="blocked"
          @click="emit('disable')"
        >
          {{ t('collaboration.disable') }}
        </BaseButton>
      </template>
    </CollabSection>
  </div>
</template>
