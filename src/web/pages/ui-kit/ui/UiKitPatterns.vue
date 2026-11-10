<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useMediaQuery } from '@/shared/lib/media-query.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import BaseStatusAction from '@/shared/ui/status/BaseStatusAction.vue';
import BaseFloatingPanel from '@/shared/ui/floating-panel/BaseFloatingPanel.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import SettingsRow from '@/shared/ui/settings-row/SettingsRow.vue';
import CollectionToolbar from '@/shared/ui/collection-toolbar/CollectionToolbar.vue';
import MasterDetailLayout from '@/shared/ui/layout/MasterDetailLayout.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
const { t } = useI18n();
const checked = ref(false); const name = ref(''); const search = ref(''); const selected = ref(1);
const detailOpen = ref(false); const floating = ref(false); const saved = ref(false); const size = ref(280);
const narrow = useMediaQuery('(width < 900px)');
const items = computed(() => [1, 2, 3].filter(number => t('designKit.item', { number }).toLowerCase().includes(search.value.toLowerCase())));
</script>
<template>
  <section class="kit-patterns">
    <h1>{{ t('designKit.title') }}</h1><p>{{ t('designKit.description') }}</p>
    <BasePanel :title="t('designKit.status')">
      <div class="kit-patterns__states">
        <BaseStatus
          :label="t('designKit.ready')"
          tone="success"
        />
        <BaseStatus
          :label="t('designKit.warning')"
          tone="warning"
        />
        <BaseStatus
          :label="t('designKit.loading')"
          tone="info"
        />
        <BaseStatusAction
          :label="t('designKit.failed')"
          :action-label="t('designKit.action')"
          tone="error"
          @click="floating = true"
        />
        <BaseStatusAction
          :label="t('designKit.disabled')"
          :action-label="t('designKit.action')"
          disabled
        />
      </div>
    </BasePanel>
    <BasePanel :title="t('designKit.settings')">
      <SettingsRow
        :label="t('designKit.enabled')"
        :description="t('designKit.enabledHint')"
        for="kit-enabled"
      >
        <BaseCheckbox
          id="kit-enabled"
          v-model="checked"
          :aria-label="t('designKit.enabled')"
        />
      </SettingsRow>
      <BaseFormRow
        v-slot="{ controlAttrs }"
        for="kit-name"
        :label="t('designKit.field')"
        :hint="t('designKit.hint')"
        :error="name.trim() ? undefined : t('designKit.invalid')"
        stacked
        required
      >
        <BaseTextField
          v-bind="controlAttrs"
          v-model="name"
        />
      </BaseFormRow>
    </BasePanel>
    <BasePanel
      :title="t('designKit.collection')"
      flush
    >
      <CollectionToolbar
        :label="t('designKit.collection')"
        :count="t('designKit.count', { count: items.length })"
      >
        <template #search>
          <BaseTextField
            v-model="search"
            type="search"
            :placeholder="t('designKit.search')"
            :aria-label="t('designKit.search')"
          />
        </template>
      </CollectionToolbar>
      <MasterDetailLayout
        v-model:size="size"
        class="kit-patterns__workspace"
        :narrow="narrow"
        :detail-open="detailOpen"
        :back-label="t('designKit.back')"
        :detail-title="t('designKit.details')"
        @back="detailOpen = false"
      >
        <template #master>
          <div class="kit-patterns__items">
            <BaseButton
              v-for="number in items"
              :key="number"
              :pressed="selected === number"
              @click="selected = number; detailOpen = true; saved = false"
            >
              {{ t('designKit.item', { number }) }}
            </BaseButton>
            <BaseEmptyState
              v-if="!items.length"
              :title="t('designKit.empty')"
            />
          </div>
        </template>
        <template #detail>
          <div class="kit-patterns__detail">
            <h2>{{ t('designKit.item', { number: selected }) }}</h2><p>{{ t('designKit.hint') }}</p><BaseButton @click="saved = true">
              {{ t('designKit.save') }}
            </BaseButton><p role="status">
              {{ saved ? t('designKit.saved') : '' }}
            </p>
          </div>
        </template>
      </MasterDetailLayout>
    </BasePanel>
    <BaseButton @click="floating = true">
      {{ t('designKit.open') }}
    </BaseButton>
    <BaseFloatingPanel
      v-model:open="floating"
      :title="t('designKit.floating')"
      :move-label="t('designKit.move')"
      :close-label="t('designKit.close')"
      :minimize-label="t('designKit.minimize')"
      :restore-label="t('designKit.restore')"
      :reset-label="t('designKit.reset')"
    >
      <p class="kit-patterns__detail">
        {{ t('designKit.description') }}
      </p>
    </BaseFloatingPanel>
  </section>
</template>
<style scoped>
.kit-patterns { display:grid; gap:var(--space-6); padding:var(--space-6); min-width:0; }
.kit-patterns h1 { font-size:var(--font-size-lg); margin:0; }
.kit-patterns p { line-height:var(--line-height-normal); }
.kit-patterns__states { display:flex; flex-wrap:wrap; align-items:center; gap:var(--space-6); }
.kit-patterns__workspace { min-height:240px; }
.kit-patterns__items { display:flex; flex-direction:column; gap:var(--space-2); padding:var(--space-4); }
.kit-patterns__detail { padding:var(--space-6); }
.kit-patterns__detail h2 { font-size:var(--font-size-md); }
</style>
