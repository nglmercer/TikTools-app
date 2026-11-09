<script setup lang="ts">
import { computed, markRaw, type Component } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';
import { useLocaleStore } from '@/shared/i18n/locale-store.ts';
import CloudAccountDiagnostics from '@/features/cloud-account/ui/CloudAccountDiagnostics.vue';
import CloudAccountEditor from '@/features/cloud-account/ui/CloudAccountEditor.vue';
import CloudLibrary from '@/features/cloud-library/ui/CloudLibrary.vue';
import CloudSyncEditor from '@/features/cloud-sync/ui/CloudSyncEditor.vue';
import CollaborationEditor from '@/features/manage-collaboration/ui/CollaborationEditor.vue';
import AppUpdater from '@/features/app-updater/ui/AppUpdater.vue';
import LanguageSelect from '@/features/change-language/ui/LanguageSelect.vue';
import RuntimeGlobalsEditor from '@/features/edit-runtime-globals/ui/RuntimeGlobalsEditor.vue';
import PluginSourcesEditor from '@/features/manage-plugin-sources/ui/PluginSourcesEditor.vue';
import type { Locale } from '@/shared/i18n/i18n.ts';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
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
      <template v-if="section === 'general'">
        <BaseGroupBox :title="t('settings.language.group')">
          <BaseFormGrid>
            <BaseFormRow
              for="settings-language"
              :label="t('settings.language.label')"
            >
              <LanguageSelect
                id="settings-language"
                v-model="locale"
                class="settings-page__control"
              />
            </BaseFormRow>
          </BaseFormGrid>
        </BaseGroupBox>
      </template>
      <template v-else-if="section === 'account'">
        <CloudAccountEditor />
        <CloudSyncEditor />
        <CloudLibrary />
      </template>
      <template v-else-if="section === 'collaboration'">
        <CollaborationEditor />
      </template>
      <template v-else-if="section === 'updates'">
        <AppUpdater />
        <PluginSourcesEditor />
      </template>
      <template v-else>
        <BaseGroupBox
          :title="t('globals.title')"
          :description="t('globals.hint', { token: '{{ globals.key }}' })"
        >
          <RuntimeGlobalsEditor />
        </BaseGroupBox>
        <CloudAccountDiagnostics />
      </template>
    </section>
  </div>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex: 1;
  gap: var(--space-4);
  min-width: 0;
  min-height: 0;
  padding: var(--space-4);
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

@media (width < 900px) {
  .settings-page {
    flex-direction: column;
    padding: var(--space-3);
    overflow-y: auto;
  }

  /* Sections become a horizontal strip of equal tabs. */
  .settings-page__sections {
    display: flex;
    flex: none;
    gap: var(--space-1);
    width: 100%;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .settings-page__sections :deep(.base-list__item) {
    flex: 1 0 auto;
    justify-content: center;
    height: var(--control-height-lg);
  }

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
