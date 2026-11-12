<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Configuration, RemoteLimits } from '@/entities/collaboration/api/types.ts';
import SettingsDisclosure from '@/shared/ui/settings-disclosure/SettingsDisclosure.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseTextArea from '@/shared/ui/text-area/BaseTextArea.vue';
defineProps<{ blocked: boolean; changed: boolean; working: boolean; available: boolean }>();
const draft = defineModel<Configuration>({ required: true });
const allowedOrigins = defineModel<string>('allowedOrigins', { required: true });
const emit = defineEmits<{ apply: []; reset: [] }>();
const { t } = useI18n();
const limits: Array<{ key: keyof RemoteLimits; min: number; max: number }> = [
  { key: 'connections', min: 1, max: 8 }, { key: 'connectionsPerPrincipal', min: 1, max: 4 },
  { key: 'frameBytes', min: 4096, max: 1048576 }, { key: 'paramsBytes', min: 1024, max: 262144 },
  { key: 'pendingCalls', min: 1, max: 32 }, { key: 'requestsPerSecond', min: 1, max: 30 },
  { key: 'requestBurst', min: 1, max: 60 }, { key: 'commandIdsPerPrincipal', min: 1, max: 100000 },
];
function setText(key: 'bindAddress' | 'advertisedOrigin' | 'tlsCertificate' | 'tlsPrivateKey', value: string): void {
  if (!draft.value) return;
  if (key === 'bindAddress') draft.value.bindAddress = value;
  else draft.value[key] = value || null;
}
function setLimit(key: keyof RemoteLimits, event: Event): void { if (draft.value) draft.value.limits[key] = (event.target as HTMLInputElement).valueAsNumber; }
function setBoolean(key: 'enabled' | 'tlsProxy', value: boolean): void { if (draft.value) draft.value[key] = value; }


</script>

<template>
  <section
    class="collaboration-editor__direct"
  >
    <BaseGroupBox
      :title="t('collaboration.configuration')"
    >
      <form
        autocomplete="off"
        @submit.prevent="emit('apply')"
      >
        <section class="collaboration-editor__section">
          <BaseSwitch
            :model-value="draft?.enabled ?? false"
            :disabled="blocked || !available"
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
                class="collaboration-editor__limit"
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
            :disabled="blocked || changed || !available"
          >
            {{ t('collaboration.save') }}
          </BaseButton>
          <BaseButton
            :disabled="working"
            @click="emit('reset')"
          >
            {{ t('collaboration.reset') }}
          </BaseButton>
        </div>
      </form>
    </BaseGroupBox>
  </section>
</template>
