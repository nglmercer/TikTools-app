<script setup lang="ts">
import { computed, markRaw, type Component } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';
import { useLocaleStore } from '@/shared/i18n/locale-store.ts';
import CloudAccountDiagnostics from '@/features/cloud-account/ui/CloudAccountDiagnostics.vue';
import CloudAccountEditor from '@/features/cloud-account/ui/CloudAccountEditor.vue';
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
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconUser from '@/shared/ui/icon/icons/IconUser.vue';
import BaseList from '@/shared/ui/list/BaseList.vue';

type SettingsSection = 'general' | 'account' | 'collaboration' | 'updates' | 'advanced';

const sections: ReadonlyArray<{ id: SettingsSection; icon: Component }> = [
  { id: 'general', icon: markRaw(IconGear) },
  { id: 'account', icon: markRaw(IconUser) },
  { id: 'collaboration', icon: markRaw(IconUser) },
  { id: 'updates', icon: markRaw(IconRefresh) },
  { id: 'advanced', icon: markRaw(IconPuzzle) },
];

/** Older deep links (`?section=globals|plugins`) now live under Advanced. */
const legacySections: Record<string, SettingsSection> = { globals: 'advanced', plugins: 'advanced' };

const { t } = useI18n();
const route = useRoute();
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
      >{{ t(`settings.sections.${item.id}`) }}</option></select>
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
        {{ t(`settings.sections.${item.id}`) }}
      </template>
    </BaseList>
    <section
      class="settings-page__content"
      :aria-label="t(`settings.sections.${section}`)"
    >
      <header class="settings-page__heading">
        <h1>{{ t(`settings.sections.${section}`) }}</h1><p>{{ t(`settingsLayout.${section}`) }}</p>
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
      </template>
      <template v-else-if="section === 'account'">
        <CloudAccountEditor />
        <CloudSyncEditor />
      </template>
      <template v-else-if="section === 'collaboration'">
        <CollaborationEditor />
      </template>
      <template v-else-if="section === 'updates'">
        <AppUpdater />
        <PluginSourcesEditor part="updates" />
      </template>
      <template v-else>
        <details
          class="settings-page__advanced"
          open
        >
          <summary>{{ t('settingsLayout.sources') }}</summary><PluginSourcesEditor part="registries" />
        </details>
        <details
          class="settings-page__advanced"
          :open="route.query.section === 'globals'"
        >
          <summary>{{ t('settingsLayout.variables') }}</summary>
          <BaseGroupBox
            :title="t('globals.title')"
            :description="t('globals.hint', { token: '{{ globals.key }}' })"
          >
            <RuntimeGlobalsEditor />
          </BaseGroupBox>
        </details>
        <details class="settings-page__advanced">
          <summary>{{ t('settingsLayout.diagnostics') }}</summary><CloudAccountDiagnostics />
        </details>
      </template>
    </section>
  </div>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex: 1;
  gap: var(--space-7);
  min-width: 0;
  min-height: 0;
  padding: var(--space-6);
}

.settings-page__sections {
  flex: none;
  width: 200px;
  border: var(--border-width) solid var(--color-border-subtle);
}

.settings-page__content {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-5);
  min-width: 0;
  max-width: 760px;
  overflow-y: auto;
}

.settings-page__control {
  max-width: 320px;
}

.settings-page__mobile-nav { display:none; }
.settings-page__mobile-nav select { width:100%; min-height:var(--control-height-touch); padding:var(--space-4); color:var(--color-text); background:var(--color-bg-field); border:var(--border-width) solid var(--color-border); border-radius:var(--radius-md); }
.settings-page__heading h1 { margin:0; font-size:20px; }
.settings-page__heading p { margin:var(--space-4) 0 var(--space-6); color:var(--color-text-muted); line-height:var(--line-height-normal); }
.settings-page__content > :deep(.base-group-box), .settings-page__advanced :deep(.base-group-box) { border:0; border-top:var(--border-width) solid var(--color-border); padding:var(--space-6) 0; background:transparent; }
.settings-page__advanced { border-top:var(--border-width) solid var(--color-border); }
.settings-page__advanced > summary { min-height:44px; padding:var(--space-5) 0; cursor:pointer; font-weight:var(--font-weight-semibold); }
.settings-page__advanced :deep(.base-group-box__title) { font-size:var(--font-size-md); }
.settings-page__advanced[open] { padding-bottom:var(--space-6); }
.settings-page__content :deep(.settings-row__control) { max-width:100%; }
@media (width < 900px) {
  .settings-page {
    flex-direction: column;
    padding: var(--space-3);
    overflow-y: auto;
  }

  .settings-page__sections { display: none; }
  .settings-page__mobile-nav { display: grid; gap: var(--space-3); }

  .settings-page__content {
    flex: none;
    width: 100%;
    max-width: none;
    overflow: visible;
  }

  .settings-page__control {
    max-width: none;
  }
}
</style>
