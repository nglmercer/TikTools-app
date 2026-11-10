<script setup lang="ts">
import { computed, markRaw, type Component } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';
import { useLocaleStore } from '@/shared/i18n/locale-store.ts';
import CloudAccountDiagnostics from '@/features/cloud-account/ui/CloudAccountDiagnostics.vue';
import CloudAccountEditor from '@/features/cloud-account/ui/CloudAccountEditor.vue';
import CloudLibrary from '@/features/cloud-library/ui/CloudLibrary.vue';
import CloudConnections from '@/features/cloud-integrations/ui/CloudConnections.vue';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import CloudSyncEditor from '@/features/cloud-sync/ui/CloudSyncEditor.vue';
import CollaborationEditor from '@/features/manage-collaboration/ui/CollaborationEditor.vue';
import AppUpdater from '@/features/app-updater/ui/AppUpdater.vue';
import LanguageSelect from '@/features/change-language/ui/LanguageSelect.vue';
import RuntimeGlobalsEditor from '@/features/edit-runtime-globals/ui/RuntimeGlobalsEditor.vue';
import PluginSourcesEditor from '@/features/manage-plugin-sources/ui/PluginSourcesEditor.vue';
import type { Locale } from '@/shared/i18n/i18n.ts';
import SettingsRow from '@/shared/ui/settings-row/SettingsRow.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import SettingsDisclosure from '@/shared/ui/settings-disclosure/SettingsDisclosure.vue';
import IconUser from '@/shared/ui/icon/icons/IconUser.vue';
import BaseList from '@/shared/ui/list/BaseList.vue';

type SettingsSection = 'general' | 'plugins' | 'account' | 'collaboration';

const sections: ReadonlyArray<{ id: SettingsSection; icon: Component }> = [
  { id: 'general', icon: markRaw(IconGear) },
  { id: 'plugins', icon: markRaw(IconPuzzle) },
  { id: 'account', icon: markRaw(IconUser) },
  { id: 'collaboration', icon: markRaw(IconUser) },
];

/** Preserve older links after regrouping options by their actual feature. */
const legacySections: Record<string, SettingsSection> = { globals: 'general', updates: 'general', advanced: 'plugins' };

const { t } = useI18n();
const route = useRoute();
const capabilities = useCapabilityStore();
// Connections appear only when the host announces them; an unknown answer keeps the panel hidden.
const connectionsAvailable = computed(() => capabilities.supports('integrations.connections') === true);
const router = useRouter();
const localeStore = useLocaleStore();

const section = computed<SettingsSection>({
  get: () => {
    const requested = String(route.query.section ?? '');
    return sections.find((entry) => entry.id === requested)?.id ?? legacySections[requested] ?? 'general';
  },
  set: (value) => void router.replace({ query: { ...route.query, section: value } }),
});

const locale = computed<Locale>({
  get: () => localeStore.locale,
  set: (value) => void localeStore.setLocale(value),
});
</script>

<template>
  <div class="settings-page">
    <label class="settings-page__mobile-nav">
      <span>{{ t('settingsLayout.section') }}</span>
      <select v-model="section"><option
        v-for="item in sections"
        :key="item.id"
        :value="item.id"
      >{{ t(`settingsLayout.labels.${item.id}`) }}</option></select>
    </label>
    <BaseList
      v-model:selected="section"
      class="settings-page__sections"
      :items="sections"
      :item-key="(entry) => entry.id"
      :aria-label="t('settings.title')"
    >
      <template #default="{ item }">
        <component :is="item.icon" />
        <span class="settings-page__nav-text">{{ t(`settingsLayout.labels.${item.id}`) }}<small>{{ t(`settingsLayout.nav.${item.id}`) }}</small></span>
      </template>
    </BaseList>
    <section
      class="settings-page__content"
      :aria-label="t(`settingsLayout.labels.${section}`)"
    >
      <header class="settings-page__heading">
        <h1>{{ t(`settingsLayout.labels.${section}`) }}</h1><p>{{ t(`settingsLayout.${section}`) }}</p>
      </header>
      <template v-if="section === 'general'">
        <BaseGroupBox :title="t('settings.language.group')">
          <SettingsRow
            for="settings-language"
            :label="t('settings.language.label')"
          >
            <LanguageSelect
              id="settings-language"
              v-model="locale"
              class="settings-page__control"
            />
          </SettingsRow>
        </BaseGroupBox>
        <AppUpdater />
        <SettingsDisclosure
          :title="t('settingsLayout.variables')"
          :initially-open="route.query.section === 'globals'"
        >
          <p
            class="settings-page__hint"
            v-text="t('globals.hint', { token: '{{ globals.key }}' })"
          />
          <RuntimeGlobalsEditor />
        </SettingsDisclosure>
      </template>
      <template v-else-if="section === 'plugins'">
        <PluginSourcesEditor />
      </template>
      <template v-else-if="section === 'account'">
        <CloudAccountEditor />
        <CloudSyncEditor />
        <CloudLibrary />
        <CloudConnections v-if="connectionsAvailable" />
        <SettingsDisclosure :title="t('settingsLayout.diagnostics')">
          <CloudAccountDiagnostics />
        </SettingsDisclosure>
      </template>
      <template v-else-if="section === 'collaboration'">
        <CollaborationEditor />
      </template>
    </section>
  </div>
</template>

<style scoped>
.settings-page { display: flex; flex: 1; gap: var(--space-5); min-width: 0; min-height: 0; padding: var(--space-4); }
.settings-page__sections { flex: none; width: 210px; align-self: flex-start; border: var(--border-width) solid var(--color-border-subtle); }
.settings-page__sections :deep(.base-list__item) { height: auto; min-height: 58px; align-items: center; padding: var(--space-4); }
.settings-page__nav-text { display: grid; gap: var(--space-1); }
.settings-page__nav-text small { color: var(--color-text-muted); font-size: var(--font-size-sm); font-weight: normal; }
.settings-page__sections :deep([aria-selected="true"]) small { color: inherit; }
.settings-page__content { display: flex; flex: 1; flex-direction: column; gap: var(--space-5); min-width: 0; max-width: 960px; overflow-y: auto; padding: 0 var(--space-2) var(--space-5); }
.settings-page__control { max-width: 320px; }
.settings-page__content :deep(.settings-row:not(.settings-row--toggle)) { grid-template-columns: minmax(0, 1fr) minmax(180px, 240px); }
.settings-page__content :deep(.settings-row__control > .base-combo-box), .settings-page__content :deep(.settings-row__control > .base-spin-box) { width: 100%; }
.settings-page__content :deep(.settings-row--toggle .settings-row__control) { justify-content: flex-end; }
.settings-page__content :deep(.base-spin-box) { min-width: 110px; }
.settings-page__mobile-nav { display: none; }
.settings-page__mobile-nav select { width: 100%; min-height: 44px; padding: var(--space-4); color: var(--color-text); background: var(--color-bg-field); border: var(--border-width) solid var(--color-border); border-radius: var(--radius-sm); }
.settings-page__heading { padding: var(--space-3) 0; }
.settings-page__heading h1 { margin: 0; font-size: 18px; }
.settings-page__heading p,.settings-page__hint { margin: var(--space-3) 0; color: var(--color-text-muted); line-height: var(--line-height-normal); }
.settings-page__content :deep(.base-group-box) { margin: 0; border-radius: 0; background: var(--color-bg-panel); }
.settings-page__content :deep(.settings-row__control) { max-width: 100%; }
.settings-page__content :deep(.settings-disclosure .base-group-box) { border: 0; padding: 0; background: transparent; }
.settings-page__content :deep(.base-switch) { min-height: 32px; }
@media (width < 900px) {
  .settings-page { flex-direction: column; padding: var(--space-3); overflow-y: auto; }
  .settings-page__sections { display: none; }
  .settings-page__mobile-nav { display: grid; gap: var(--space-3); }
  .settings-page__content { flex: none; width: 100%; max-width: none; overflow: visible; padding: 0 0 var(--space-5); }
  .settings-page__control { max-width: none; }
  .settings-page__content :deep(.settings-row:not(.settings-row--toggle)) { grid-template-columns: minmax(0, 1fr); }
  .settings-page__content :deep(.settings-row__control > .base-spin-box) { max-width: 160px; }
  .settings-page__content :deep(.base-switch) { min-height: 44px; }
}
</style>
