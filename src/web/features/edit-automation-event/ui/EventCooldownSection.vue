<script setup lang="ts">
import { computed } from 'vue';
import type { LiveEvent } from '../../../../automation/behavior/types.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseSection from '@/shared/ui/section/BaseSection.vue';
import BaseSpinBox from '@/shared/ui/spin-box/BaseSpinBox.vue';
import IconClock from '@/shared/ui/icon/icons/IconClock.vue';
import IconGlobe from '@/shared/ui/icon/icons/IconGlobe.vue';
import IconUser from '@/shared/ui/icon/icons/IconUser.vue';
const props = defineProps<{ event: LiveEvent; fieldId: string }>();
const emit = defineEmits<{ change: [patch: Partial<Pick<LiveEvent, 'cooldownMs' | 'cooldownScope'>>] }>();
const { t } = useAutomationText();
// Cooldown is edited in whole seconds; the record keeps milliseconds.
const cooldownSeconds = computed({
  get: () => Math.round(props.event.cooldownMs / 1000),
  set: (seconds: number) => emit('change', { cooldownMs: Math.max(0, seconds) * 1000 }),
});
</script>

<template>
  <BaseSection :title="t('automations.event.cooldown')">
    <BaseSpinBox
      :id="`${fieldId}-cooldown`"
      v-model="cooldownSeconds"
      orientation="horizontal"
      :min="0"
      :max="86400"
      suffix="s"
      :aria-label="t('automations.event.cooldown')"
    >
      <template #prefix>
        <IconClock />
      </template>
    </BaseSpinBox>
    <div
      v-if="event.cooldownMs > 0"
      class="event-editor__scope"
      role="group"
      :aria-label="t('automations.event.scope')"
    >
      <BaseIconButton
        :label="t('automations.event.scopeUser')"
        variant="default"
        size="sm"
        :pressed="event.cooldownScope === 'user'"
        @click="emit('change', { cooldownScope: 'user' })"
      >
        <IconUser />
      </BaseIconButton>
      <BaseIconButton
        :label="t('automations.event.scopeGlobal')"
        variant="default"
        size="sm"
        :pressed="event.cooldownScope === 'global'"
        @click="emit('change', { cooldownScope: 'global' })"
      >
        <IconGlobe />
      </BaseIconButton>
    </div>
  </BaseSection>
</template>
