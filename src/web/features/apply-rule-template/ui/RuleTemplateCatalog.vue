<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { BEHAVIOR_TRIGGERS } from '../../../../automation/behavior/schema.ts';
import type { BehaviorSnapshot } from '../../../../automation/behavior/types.ts';
import { availableActionTypeIds, matchesQuery } from '@/entities/automation/lib/automation-records.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import { BUILTIN_RULE_TEMPLATES, missingRuleTemplateRequirements, type RuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
const props = defineProps<{ snapshot: BehaviorSnapshot; customTemplates: readonly RuleTemplate[]; customError?: string | null }>();
const emit = defineEmits<{ configure: [template: RuleTemplate] }>();
const query = defineModel<string>('query', { default: '' });
const { t } = useI18n();
const { triggerName } = useAutomationText();
const templates = computed(() => [...BUILTIN_RULE_TEMPLATES, ...props.customTemplates]);
const availableTypes = computed(() => availableActionTypeIds(props.snapshot.plugins, props.snapshot.actionTypes));
const knownTriggers = computed(() => [...BEHAVIOR_TRIGGERS, ...(props.snapshot.eventTypes ?? []).map((entry) => entry.type)]);
const entries = computed(() =>
  templates.value
    .filter((template) =>
      matchesQuery(query.value, localizedText(template.title), localizedText(template.description), template.event.trigger),
    )
    .map((template) => {
      const requirements = missingRuleTemplateRequirements(template, availableTypes.value, knownTriggers.value);
      const problems = [
        ...(requirements.missingTypeIds.length ? [t('automations.templates.requires', { name: requirements.missingTypeIds.join(', ') })] : []),
        ...(requirements.unknownTrigger ? [t('automations.templates.unknownTrigger', { name: template.event.trigger })] : []),
      ];
      return { template, problems };
    }),
);
</script>

<template>
  <p class="rule-templates__lead">
    {{ t('automations.templates.lead') }}
  </p>
  <div class="rule-templates__toolbar">
    <BaseTextField
      v-model="query"
      class="rule-templates__search"
      type="search"
      :placeholder="t('automations.templates.search')"
      :aria-label="t('automations.templates.search')"
    >
      <template #prefix>
        <IconSearch />
      </template>
    </BaseTextField>
  </div>
  <p
    v-if="customError"
    class="rule-templates__error"
    role="alert"
  >
    {{ customError }}
  </p>
  <p
    v-if="!entries.length"
    class="rule-templates__muted"
  >
    {{ t('automations.templates.empty') }}
  </p>
  <div class="rule-templates__grid">
    <button
      v-for="entry in entries"
      :key="`${entry.template.custom ? 'custom' : 'builtin'}-${entry.template.id}`"
      class="rule-templates__card"
      type="button"
      :disabled="entry.problems.length > 0"
      @click="emit('configure', entry.template)"
    >
      <span class="rule-templates__card-title">
        {{ localizedText(entry.template.title) }}
        <span
          v-if="entry.template.custom"
          class="rule-templates__badge"
        >{{ t('automations.templates.imported') }}</span>
      </span>
      <span class="rule-templates__card-description">{{ localizedText(entry.template.description) }}</span>
      <span class="rule-templates__card-trigger">{{ triggerName(entry.template.event.trigger, snapshot.eventTypes ?? []) }}</span>
      <span
        v-for="problem in entry.problems"
        :key="problem"
        class="rule-templates__problem"
      >{{ problem }}</span>
    </button>
  </div>
</template>
