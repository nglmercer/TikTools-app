<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PluginCredentialState, TokenSetupOptions } from '../../../../shared/token-import.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconFile from '@/shared/ui/icon/icons/IconFile.vue';
import IconFolder from '@/shared/ui/icon/icons/IconFolder.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';

/**
 * Credential actions of a plugin connection: automatic discovery, folder or file import,
 * the candidate list, the import result and the authorization entry point. It only
 * renders state and reports intent; the editor owns the values, the single autosave and
 * the one provisioning dialog, so this block never loads, saves, tests or opens anything.
 */
const props = defineProps<{
  /** `stacked` is the full-width plugin page; `sections` is the compact server panel. */
  variant: 'stacked' | 'sections';
  supportsTokenImport?: boolean;
  supportsTokenDiscovery?: boolean;
  /** Whether the authorization button is offered (decided by the editor from the connection state). */
  authorizable: boolean;
  /** Guided connections label the button "Authorize"; the others "Get token". */
  guided: boolean;
  working: boolean;
  disabled: boolean;
  state?: PluginCredentialState;
}>();

const emit = defineEmits<{ import: [options?: TokenSetupOptions]; authorize: [] }>();
const { t } = useI18n();
const compact = computed(() => props.variant === 'sections');
</script>

<template>
  <div class="credentials">
    <component
      :is="compact ? 'div' : 'section'"
      v-if="supportsTokenImport"
      :class="compact ? 'credentials__import' : 'credentials__token'"
    >
      <div :class="compact ? 'credentials__row' : 'credentials__actions'">
        <template v-if="supportsTokenDiscovery">
          <BaseButton
            variant="primary"
            :disabled="disabled"
            :title="t('plugins.connection.autoConnectHint')"
            @click="emit('import', { mode: 'auto' })"
          >
            <template #icon>
              <IconGear v-if="compact" />
              <IconSearch v-else />
            </template>
            {{ working ? t('plugins.connection.importing') : t(compact ? 'plugins.connection.actions.auto' : 'plugins.connection.autoConnect') }}
          </BaseButton>
          <BaseButton
            :disabled="disabled"
            :title="t('plugins.connection.autoConnectHint')"
            @click="emit('import', { mode: 'folder' })"
          >
            <template #icon>
              <IconFolder />
            </template>
            {{ t(compact ? 'plugins.connection.actions.folder' : 'plugins.connection.selectFolder') }}
          </BaseButton>
        </template>
        <BaseButton
          :disabled="disabled"
          :title="t('plugins.connection.importTokenHint')"
          @click="emit('import')"
        >
          <template #icon>
            <IconFile v-if="compact" />
            <IconDownload v-else />
          </template>
          {{ working ? t('plugins.connection.importing') : t(compact ? 'plugins.connection.actions.import' : 'plugins.connection.importToken') }}
        </BaseButton>
      </div>
    </component>

    <div
      v-if="state?.candidates?.length"
      class="credentials__candidates"
    >
      <p class="credentials__note">
        {{ t('plugins.connection.chooseConfiguration') }}
      </p>
      <ul class="credentials__candidate-list">
        <li
          v-for="candidate in state.candidates"
          :key="candidate.id"
        >
          <span class="credentials__candidate">{{ candidate.label }}</span>
          <BaseButton
            size="sm"
            :disabled="disabled"
            @click="emit('import', { mode: 'candidate', candidateId: candidate.id })"
          >
            {{ t('plugins.connection.useConfiguration') }}
          </BaseButton>
        </li>
      </ul>
    </div>
    <p
      v-if="state?.message"
      class="credentials__details"
      :class="{ 'credentials__details--ok': state.ok }"
      role="status"
    >
      {{ state.message }}
    </p>

    <div
      v-if="authorizable"
      class="credentials__actions"
    >
      <BaseButton
        :disabled="disabled"
        @click="emit('authorize')"
      >
        {{ t(guided ? 'plugins.connection.authorize' : 'plugins.connection.getToken') }}
      </BaseButton>
    </div>
  </div>
</template>

<style scoped>
/* The block lays out as if its parts were direct children of the editor or its section. */
.credentials {
  display: contents;
}

.credentials__actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
}

.credentials__row {
  display: flex;
  gap: var(--space-2);
}

.credentials__row :deep(.base-button) {
  flex: 1 1 0;
  min-width: 0;
  padding: 0 var(--space-3);
}

.credentials__token {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding-top: var(--space-4);
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.credentials__note {
  margin: 0;
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
  line-height: var(--line-height-normal);
}

.credentials__details {
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-danger);
  border-radius: var(--radius-sm);
  color: var(--color-danger-hover);
  overflow-wrap: anywhere;
}

.credentials__details--ok {
  border-color: var(--color-success);
  color: var(--color-success);
}

.credentials__candidate-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  max-height: 20rem;
  margin: var(--space-2) 0 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
}

.credentials__candidate-list li {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}

.credentials__candidate {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
}
</style>
