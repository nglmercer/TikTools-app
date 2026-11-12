<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { computed } from 'vue';
import type { CollaborationStatus } from '@/entities/collaboration/api/types.ts';
import { useCollaborationPresentation } from '../lib/use-collaboration-presentation.ts';
import SettingsDisclosure from '@/shared/ui/settings-disclosure/SettingsDisclosure.vue';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
const props = defineProps<{ status: CollaborationStatus; checkedAt: number | null; blocked: boolean }>();
const emit = defineEmits<{ disable: [] }>();
const { t } = useI18n();
const { date, listenerStatus } = useCollaborationPresentation();
const listener = computed(() => listenerStatus(props.status));
</script>

<template>
  <BaseGroupBox
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
        @click="emit('disable')"
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
</template>
