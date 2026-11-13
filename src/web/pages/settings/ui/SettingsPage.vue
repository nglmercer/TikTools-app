<script setup lang="ts">
import { computed, markRaw } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';
import { useLocaleStore } from '@/shared/i18n/locale-store.ts';
import CloudAccountDiagnostics from '@/features/cloud-account/ui/CloudAccountDiagnostics.vue';
import CloudAccountEditor from '@/features/cloud-account/ui/CloudAccountEditor.vue';
import CloudTeams from '@/features/cloud-teams/ui/CloudTeams.vue';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import CollaborationEditor from '@/features/manage-collaboration/ui/CollaborationEditor.vue';
import AppUpdater from '@/features/app-updater/ui/AppUpdater.vue';
import LanguageSelect from '@/features/change-language/ui/LanguageSelect.vue';
import type { Locale } from '@/shared/i18n/i18n.ts';
import SettingsRow from '@/shared/ui/settings-row/SettingsRow.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconGlobe from '@/shared/ui/icon/icons/IconGlobe.vue';
import IconUser from '@/shared/ui/icon/icons/IconUser.vue';
import SettingsDisclosure from '@/shared/ui/settings-disclosure/SettingsDisclosure.vue';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';

/**
 * Settings keeps what belongs to the application itself. Plugin preferences, overlays, webhooks,
 * variables and cloud documents live in the area they configure.
 */
type SettingsSection = 'general' | 'account' | 'collaboration' | 'teams';

const sections: ReadonlyArray<{ id: SettingsSection; icon: typeof IconGear }> = [
  { id: 'general', icon: markRaw(IconGear) },
  { id: 'account', icon: markRaw(IconGlobe) },
  { id: 'collaboration', icon: markRaw(IconUser) },
  { id: 'teams', icon: markRaw(IconUser) },
];

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const capabilities = useCapabilityStore();
const localeStore = useLocaleStore();
// Teams are part of remote collaboration; without the host's announcement they stay hidden.
const teamsAvailable = computed(() => capabilities.supports('teams.hosts.list') === true);

const tabs = computed(() => sections.filter(entry => entry.id !== 'teams' || teamsAvailable.value).map((entry) => ({ value: entry.id, label: entry.id === 'teams' ? t('cloudTeams.title') : t(`settingsLayout.labels.${entry.id}`), icon: entry.icon })));

// An unknown (or moved) section lands on General; the router redirects the links of moved areas.
const section = computed<SettingsSection>({
  get: () => sections.find((entry) => entry.id === String(route.query.section ?? '') && (entry.id !== 'teams' || teamsAvailable.value))?.id ?? 'general',
  set: (value) => void router.replace({ query: { ...route.query, section: value } }),
});

const locale = computed<Locale>({
  get: () => localeStore.locale,
  set: (value) => void localeStore.setLocale(value),
});
</script>

<template>
  <div class="settings-page">
    <BaseTabs
      v-model="section"
      class="settings-page__tabs"
      :tabs="tabs"
    >
      <section
        class="settings-page__content"
        :aria-label="section === 'teams' ? t('cloudTeams.title') : t(`settingsLayout.labels.${section}`)"
      >
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
        </template>
        <template v-else-if="section === 'account'">
          <CloudAccountEditor />
          <SettingsDisclosure :title="t('settingsLayout.diagnostics')">
            <CloudAccountDiagnostics />
          </SettingsDisclosure>
        </template>
        <CollaborationEditor
          v-else-if="section === 'collaboration'"
          @teams="section = 'teams'"
        />
        <CloudTeams v-else-if="teamsAvailable" />
      </section>
    </BaseTabs>
  </div>
</template>

<style scoped>
.settings-page { display: flex; flex: 1; flex-direction: column; min-width: 0; min-height: 0; padding: var(--space-4); }
.settings-page__tabs { flex: 1; min-height: 0; }
.settings-page__tabs :deep(.base-tabs__panel) { display: flex; flex-direction: column; }
.settings-page__content { display: flex; flex: 1; flex-direction: column; gap: var(--space-6); min-width: 0; max-width: 1120px; min-height: 0; overflow-y: auto; padding: var(--space-5) var(--space-2) var(--space-5) 0; }
.settings-page__control { max-width: 320px; }
.settings-page__content :deep(.settings-row:not(.settings-row--toggle)) { grid-template-columns: minmax(0, 1fr) minmax(180px, 240px); }
.settings-page__content :deep(.settings-row__control > .base-combo-box), .settings-page__content :deep(.settings-row__control > .base-spin-box), .settings-page__content :deep(.settings-row__control > .base-search-select) { width: 100%; }
.settings-page__content :deep(.settings-row--toggle .settings-row__control) { justify-content: flex-end; }
.settings-page__content :deep(.base-spin-box) { min-width: 110px; }
.settings-page__content :deep(.base-group-box) { margin: 0; border-radius: 0; background: var(--color-bg-panel); }
.settings-page__content :deep(.settings-row__control) { max-width: 100%; }
.settings-page__content :deep(.settings-disclosure .base-group-box) { border: 0; padding: 0; background: transparent; }
.settings-page__content :deep(.base-switch) { min-height: 32px; }
@media (width < 900px) {
  .settings-page { padding: var(--space-3); overflow-y: auto; }
  .settings-page__tabs { flex: none; }
  .settings-page__content { flex: none; width: 100%; max-width: none; overflow: visible; padding: var(--space-4) 0 var(--space-5); }
  .settings-page__control { max-width: none; }
  .settings-page__content :deep(.settings-row:not(.settings-row--toggle)) { grid-template-columns: minmax(0, 1fr); }
  .settings-page__content :deep(.settings-row__control > .base-spin-box) { max-width: 160px; }
  .settings-page__content :deep(.base-switch) { min-height: 44px; }
}
@media (width < 900px) {
 .settings-page__tabs :deep(.base-tabs) { min-width:0; }
 .settings-page__tabs :deep(.base-tabs__bar) { min-width:0; max-width:100%; }
 .settings-page__tabs :deep(.base-tabs__list) { flex-wrap:nowrap; overflow-x:auto; min-width:0; max-width:100%; scrollbar-width:thin; }
 .settings-page__tabs :deep(.base-tabs__tab) { flex:none; white-space:nowrap; min-height:44px; }
}
@media (width < 600px) {
 .settings-page__tabs > :deep(.base-tabs__bar > .base-tabs__list) { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); overflow:visible; }
 .settings-page__tabs > :deep(.base-tabs__bar .base-tabs__tab) { justify-content:center; padding:0 var(--space-4); font-size:var(--font-size-sm); }
}
</style>
