<script setup lang="ts">
import SettingsRow from "@/shared/ui/settings-row/SettingsRow.vue";
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AutoUpdate, RegistryDefaults, RegistryRootPreview } from '@/entities/plugin-package/api/package-types.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { formatDate } from '@/shared/lib/intl.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import SettingsDisclosure from '@/shared/ui/settings-disclosure/SettingsDisclosure.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseSpinBox from '@/shared/ui/spin-box/BaseSpinBox.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

/**
 * `updates`: plugin update policy for everyone (Settings › Updates).
 * `registries`: registry sources, sideloading and manual registry setup,
 * shown only under Settings › Advanced.
 */
const props = withDefaults(defineProps<{ part?: 'updates' | 'registries' }>(), { part: 'registries' });

const { t, locale } = useI18n();
const addOpen = ref(false);
const packages = usePackagesStore();

const name = ref('');
const url = ref('');
const scope = ref('');
const root = ref<RegistryRootPreview | null>(null);
const confirmed = ref(false);
const unverified = ref(false);
const working = ref(false);

const channelOptions = computed(() => [
  { value: 'stable', label: t('packages.policy.stable') },
  { value: 'beta', label: t('packages.policy.beta') },
]);
const autoOptions = computed(() =>
  (['off', 'notify', 'patch', 'minor'] as const).map((value) => ({ value, label: t(`packages.policy.auto.${value}`) })),
);

function update<K extends keyof RegistryDefaults>(key: K, value: RegistryDefaults[K]): void {
  if (!packages.defaults) return;
  void packages.saveDefaults({ ...packages.defaults, [key]: value });
}

const channel = computed({
  get: () => packages.defaults?.channel ?? 'stable',
  set: (value: string) => update('channel', value),
});
const autoUpdate = computed({
  get: () => packages.defaults?.autoUpdate ?? 'notify',
  set: (value: AutoUpdate) => update('autoUpdate', value),
});
const interval = computed({
  get: () => packages.defaults?.checkIntervalHours ?? 12,
  set: (value: number) => update('checkIntervalHours', Math.round(value)),
});
const allowSideload = computed({
  get: () => packages.defaults?.allowSideload ?? true,
  set: (value: boolean) => update('allowSideload', value),
});

const customRegistries = computed(() => packages.registries.filter((registry) => !registry.builtin));
const builtinRegistries = computed(() => packages.registries.filter((registry) => registry.builtin));

const canFetch = computed(() => /^[a-z][a-z0-9-]{1,31}$/.test(name.value) && /^https?:\/\//.test(url.value.trim()));
const canAdd = computed(() => canFetch.value && !working.value && (unverified.value || (root.value !== null && confirmed.value)));

async function fetchKeys(): Promise<void> {
  working.value = true;
  confirmed.value = false;
  root.value = await packages.previewRegistry(url.value.trim());
  working.value = false;
}

async function add(): Promise<void> {
  if (!canAdd.value) return;
  working.value = true;
  const added = await packages.addRegistry({
    name: name.value,
    url: url.value.trim(),
    fingerprints: unverified.value ? [] : (root.value?.keys.map((key) => key.fingerprint) ?? []),
    scopes: scope.value.trim() ? [scope.value.trim()] : [],
    priority: 0,
  });
  working.value = false;
  if (added) {
    name.value = '';
    url.value = '';
    scope.value = '';
    root.value = null;
    confirmed.value = false;
    unverified.value = false;
  }
}

onMounted(() => void packages.loadRegistries());
</script>

<template>
  <div class="plugin-sources">
    <BaseAlert
      v-if="packages.registryError"
      :title="packages.registryError"
      compact
    />
    <BaseGroupBox
      v-if="props.part === 'updates'"
      :title="t('packages.sources.updates')"
    >
      <div>
        <SettingsRow
          for="plugin-sources-channel"
          :label="t('packages.policy.channel')"
        >
          <BaseComboBox
            id="plugin-sources-channel"
            v-model="channel"
            :options="channelOptions"
            :disabled="!packages.defaults"
          />
        </SettingsRow>
        <SettingsRow
          for="plugin-sources-auto"
          :label="t('packages.policy.autoUpdate')"
          :description="t('packages.policy.autoHint')"
        >
          <BaseComboBox
            id="plugin-sources-auto"
            v-model="autoUpdate"
            :options="autoOptions"
            :disabled="!packages.defaults"
          />
        </SettingsRow>
        <SettingsRow
          for="plugin-sources-interval"
          :label="t('packages.sources.interval')"
        >
          <BaseSpinBox
            id="plugin-sources-interval"
            v-model="interval"
            :min="1"
            :max="168"
            :suffix="t('packages.sources.hours')"
            :disabled="!packages.defaults"
          />
        </SettingsRow>
      </div>
    </BaseGroupBox>

    <BaseGroupBox
      v-if="props.part === 'registries'"
      :title="t('packages.sources.registries')"
    >
      <p
        v-if="!builtinRegistries.length"
        class="plugin-sources__muted"
      >
        {{ t('packages.sources.officialUnavailable') }}
      </p>
      <ul class="plugin-sources__list">
        <li
          v-for="registry in [...builtinRegistries, ...customRegistries]"
          :key="registry.name"
          class="plugin-sources__registry"
        >
          <div class="plugin-sources__info">
            <span class="plugin-sources__name">
              {{ registry.name }}
              <span
                class="plugin-sources__badge"
                :class="{ 'plugin-sources__badge--warning': !registry.verified }"
              >{{ registry.verified ? t('packages.sources.verified') : t('packages.sources.unverified') }}</span>
            </span>
            <span class="plugin-sources__mono">{{ registry.url }}</span>
            <span
              v-if="registry.error"
              class="plugin-sources__error"
            >{{ registry.error }}</span>
            <span
              v-else-if="registry.refreshedAt"
              class="plugin-sources__muted"
            >{{ t('packages.sources.refreshed', { date: formatDate(registry.refreshedAt * 1000, locale) }) }}</span>
          </div>
          <BaseSwitch
            :model-value="registry.enabled"
            @update:model-value="(value: boolean) => packages.setRegistryEnabled(registry.name, value)"
          >
            {{ t('packages.sources.enabled') }}
          </BaseSwitch>
          <BaseButton
            size="sm"
            @click="packages.refreshRegistries(registry.name)"
          >
            {{ t('packages.sources.refresh') }}
          </BaseButton>
          <BaseButton
            v-if="!registry.builtin"
            size="sm"
            variant="danger"
            @click="packages.removeRegistry(registry.name)"
          >
            {{ t('packages.sources.remove') }}
          </BaseButton>
        </li>
      </ul>
      <BaseSwitch
        v-model="allowSideload"
        :disabled="!packages.defaults"
      >
        {{ t('packages.sources.allowSideload') }}
      </BaseSwitch>
      <SettingsDisclosure
        v-model:open="addOpen"
        :title="t('packages.sources.add')"
      >
        <p class="plugin-sources__muted">
          {{ t('packages.sources.addHint') }}
        </p>
        <BaseFormGrid>
          <BaseFormRow
            for="plugin-sources-name"
            :label="t('packages.sources.name')"
          >
            <BaseTextField
              id="plugin-sources-name"
              v-model="name"
              placeholder="acme"
            />
          </BaseFormRow>
          <BaseFormRow
            for="plugin-sources-url"
            :label="t('packages.sources.url')"
          >
            <BaseTextField
              id="plugin-sources-url"
              v-model="url"
              type="url"
              placeholder="https://plugins.example.com/v1"
            />
          </BaseFormRow>
          <BaseFormRow
            for="plugin-sources-scope"
            :label="t('packages.sources.scope')"
            :hint="t('packages.sources.scopeHint')"
          >
            <BaseTextField
              id="plugin-sources-scope"
              v-model="scope"
              placeholder="acme.*"
            />
          </BaseFormRow>
        </BaseFormGrid>
        <div class="plugin-sources__actions">
          <BaseButton
            :disabled="!canFetch || working || unverified"
            @click="fetchKeys"
          >
            {{ t('packages.sources.fetchKeys') }}
          </BaseButton>
          <BaseSwitch v-model="unverified">
            {{ t('packages.sources.unverifiedOption') }}
          </BaseSwitch>
        </div>
        <div
          v-if="root && !unverified"
          class="plugin-sources__keys"
        >
          <p class="plugin-sources__muted">
            {{ t('packages.sources.fingerprintsHint') }}
          </p>
          <ul class="plugin-sources__list">
            <li
              v-for="key in root.keys"
              :key="key.keyId"
              class="plugin-sources__mono"
            >
              {{ key.keyId }} · {{ key.fingerprint }}
            </li>
          </ul>
          <BaseSwitch v-model="confirmed">
            {{ t('packages.sources.confirmFingerprints') }}
          </BaseSwitch>
        </div>
        <BaseButton
          variant="primary"
          :disabled="!canAdd"
          @click="add"
        >
          {{ t('packages.sources.addButton') }}
        </BaseButton>
      </SettingsDisclosure>
    </BaseGroupBox>
  </div>
</template>

<style scoped>
.plugin-sources {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.plugin-sources__list {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  margin: 0;
  padding: 0;
  list-style: none;
}

.plugin-sources__registry {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-3);
  border: var(--border-width) solid var(--color-border-subtle);
  border-radius: var(--radius-sm);
}

.plugin-sources__info {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
}

.plugin-sources__name {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  font-weight: var(--font-weight-semibold);
}

.plugin-sources__badge {
  padding: 0 var(--space-2);
  border: var(--border-width) solid var(--color-success);
  border-radius: var(--radius-sm);
  color: var(--color-success);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-normal);
}

.plugin-sources__badge--warning {
  border-color: var(--color-warning);
  color: var(--color-warning);
}

.plugin-sources__mono {
  overflow-wrap: anywhere;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}

.plugin-sources__muted {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.plugin-sources__error {
  color: var(--color-danger-hover);
  font-size: var(--font-size-sm);
}

.plugin-sources__actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-4);
  margin: var(--space-3) 0;
}

.plugin-sources__keys {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  margin-bottom: var(--space-3);
}
</style>
