<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ActionTypeDefinition, PluginStatus } from '../../../../automation/behavior/types.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseMarkdown from '@/shared/ui/markdown/BaseMarkdown.vue';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconLock from '@/shared/ui/icon/icons/IconLock.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
const props = defineProps<{ descriptor: PluginStatus['descriptor']; actionTypes: readonly ActionTypeDefinition[] }>();
const { t } = useI18n();
const descriptor = computed(() => props.descriptor);
const description = computed(() => {
  const long = descriptor.value.longDescription ? localizedText(descriptor.value.longDescription) : '';
  return long || localizedText(descriptor.value.description);
});
const actionNames = computed(() =>
  descriptor.value.actionTypeIds.map((typeId) => {
    const type = props.actionTypes.find((entry) => entry.id === typeId);
    return type ? localizedText(type.title) : typeId;
  }),
);

</script>

<template>
  <div
    class="plugin-details__general"
  >
    <BaseMarkdown
      v-if="description"
      class="plugin-details__description"
      :source="description"
    />
    <div class="plugin-details__columns">
      <div class="plugin-details__column">
        <BaseSection :title="t('plugins.details.source')">
          <div
            v-tooltip="t('plugins.details.source')"
            class="plugin-details__field"
          >
            <IconLayers />
            <span class="plugin-details__value">{{ t(`plugins.details.sources.${descriptor.source ?? 'builtin'}`) }}</span>
          </div>
          <div
            v-tooltip="t('plugins.details.dependency')"
            class="plugin-details__field"
          >
            <IconPuzzle />
            <span class="plugin-details__value">{{ localizedText(descriptor.dependency) || t('plugins.details.none') }}</span>
          </div>
        </BaseSection>
        <BaseSection :title="t('plugins.details.permissions')">
          <div class="plugin-details__chips">
            <IconLock class="plugin-details__lead" />
            <span
              v-if="!descriptor.permissions.length"
              class="plugin-details__none"
            >{{ t('plugins.details.none') }}</span>
            <span
              v-for="permission in descriptor.permissions"
              :key="permission"
              class="plugin-details__chip plugin-details__mono"
            >{{ permission }}</span>
          </div>
        </BaseSection>
      </div>
      <div class="plugin-details__column">
        <BaseSection :title="t('plugins.details.actions')">
          <span
            v-if="!actionNames.length"
            class="plugin-details__none"
          >{{ t('plugins.details.none') }}</span>
          <div
            v-for="action in actionNames"
            :key="action"
            class="plugin-details__field"
          >
            <span class="plugin-details__value">{{ action }}</span>
          </div>
        </BaseSection>
        <BaseSection :title="t('plugins.details.events')">
          <span
            v-if="!descriptor.eventTypeIds.length"
            class="plugin-details__none"
          >{{ t('plugins.details.none') }}</span>
          <div
            v-for="event in descriptor.eventTypeIds"
            :key="event"
            class="plugin-details__field"
          >
            <span class="plugin-details__value plugin-details__mono">{{ event }}</span>
          </div>
        </BaseSection>
      </div>
    </div>
  </div>
</template>
