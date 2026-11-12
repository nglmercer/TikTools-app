<script setup lang="ts">
import { computed, markRaw, ref, shallowRef } from 'vue';
import { applyConditionTemplate, conditionTemplatesFor, type ConditionTemplate } from '../../../../automation/behavior/condition-templates.ts';
import { registerLiveGoalTemplates } from '../../../../automation/behavior/live-goal-templates.ts';
import { registerModerationTemplates } from '../../../../automation/behavior/moderation-templates.ts';
import type { LiveEvent } from '../../../../automation/behavior/types.ts';
import type { HotkeyStatusData } from '../../../../shared/messages.ts';
import { rawHotkeyInputHint, requiresRawHotkeyInput } from '@/entities/automation/lib/hotkey-status.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconFile from '@/shared/ui/icon/icons/IconFile.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import type { SearchOption } from '@/shared/ui/search-select/search-option.ts';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
import ConditionsEditor from './ConditionsEditor.vue';
import ConditionTemplateDialog from './ConditionTemplateDialog.vue';
const props = defineProps<{
  event: LiveEvent;
  gifts: readonly SearchOption[];
  viewers: readonly SearchOption[];
  hotkeyStatus?: HotkeyStatusData | null;
}>();
const emit = defineEmits<{ change: [filters: LiveEvent['filters']] }>();
registerModerationTemplates();
registerLiveGoalTemplates();
const { t } = useAutomationText();
const templatesOpen = ref(false);
const templatesAnchor = shallowRef<HTMLElement | null>(null);
const configuring = shallowRef<ConditionTemplate | null>(null);
const configureOpen = ref(false);

const templates = computed(() => conditionTemplatesFor(props.event.trigger));
const templateMenu = computed<MenuEntry[]>(() =>
  templates.value.map((template) => ({ id: template.id, label: localizedText(template.title) })),
);
const rawHotkey = computed(
  () =>
    props.event.trigger === 'hotkey.pressed' &&
    rawHotkeyInputHint(props.hotkeyStatus, requiresRawHotkeyInput(props.event.filters)) !== null,
);
const filters = computed({
  get: () => props.event.filters,
  set: (next) => emit('change', next),
});
function openTemplates(event: MouseEvent): void {
  templatesAnchor.value = markRaw(event.currentTarget as HTMLElement);
  templatesOpen.value = true;
}

function pickTemplate(action: MenuAction): void {
  const template = templates.value.find((entry) => entry.id === action.id);
  if (!template) return;
  if (template.params?.length) {
    configuring.value = template;
    configureOpen.value = true;
  } else {
    emit('change', applyConditionTemplate(template, {}));
  }
}

function applyConfigured(answers: Record<string, string>): void {
  if (configuring.value) emit('change', applyConditionTemplate(configuring.value, answers));
}
</script>

<template>
  <BaseSection
    :title="t('automations.event.conditions')"
    :description="rawHotkey ? t('automations.event.rawHotkey') : undefined"
  >
    <template
      v-if="templates.length"
      #actions
    >
      <BaseIconButton
        :label="t('automations.conditions.templates')"
        size="sm"
        aria-haspopup="menu"
        @click="openTemplates"
      >
        <IconFile />
      </BaseIconButton>
    </template>
    <ConditionsEditor
      v-model="filters"
      :trigger="event.trigger"
      :gifts="gifts"
      :viewers="viewers"
    />
  </BaseSection>
  <BaseMenu
    v-model:open="templatesOpen"
    :items="templateMenu"
    :anchor="templatesAnchor"
    @select="pickTemplate"
  />
  <ConditionTemplateDialog
    v-model:open="configureOpen"
    :template="configuring"
    @apply="applyConfigured"
  />
</template>
