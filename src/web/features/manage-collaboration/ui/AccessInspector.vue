<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Grant } from '@/entities/collaboration/api/types.ts';
import { splitIds } from '@/entities/collaboration/lib/grants.ts';
import { canSetLevel, levelOf, levelsFor, scopeOf, withLevel, withScope, type AccessArea, type AccessLevel } from '../model/access-areas.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

const props = defineProps<{ title: string; area: AccessArea; resetKey: string; disabled?: boolean; dirty?: boolean; saveable?: boolean }>();
const grant = defineModel<Grant>({ required: true });
const emit = defineEmits<{ apply: []; discard: [] }>();
const { t } = useI18n();
const level = computed(() => levelOf(grant.value, props.area));
const levelOptions = computed(() => levelsFor(props.area).map(value => ({ value, label: t(`collaboration.levels.${value}`), disabled: !canSetLevel(grant.value, props.area, value) })));
const lockedSomewhere = computed(() => levelOptions.value.some(option => option.disabled));
const scope = computed(() => scopeOf(grant.value, props.area));
const scopeMode = computed<'all' | 'selected'>(() => scope.value === undefined ? 'all' : 'selected');
const hasAccess = computed(() => level.value !== 'none');
// Typing keeps its own text so separators survive until the list is complete.
const idsText = ref('');
watch(() => [props.resetKey, props.area.id] as const, () => { idsText.value = (scopeOf(grant.value, props.area) ?? []).join(', '); }, { immediate: true });
const setLevel = (value: AccessLevel): void => { grant.value = withLevel(grant.value, props.area, value); };
function setMode(mode: 'all' | 'selected'): void { grant.value = withScope(grant.value, props.area, mode === 'all' ? undefined : splitIds(idsText.value.replace(/,/g, '\n'))); }
function setIds(text: string): void { idsText.value = text; grant.value = withScope(grant.value, props.area, splitIds(text.replace(/,/g, '\n'))); }
function setLimit(event: Event): void {
  const input = event.target as HTMLInputElement;
  grant.value = { ...grant.value, maxPointsDelta: input.validity.badInput ? 0 : input.value === '' ? null : Number.isFinite(input.valueAsNumber) ? input.valueAsNumber : 0 };
}
</script>

<template>
  <section
    class="access-inspector"
    :aria-label="t('collaboration.editing')"
  >
    <div class="access-inspector__title">
      <span class="collaboration-editor__muted">{{ t('collaboration.editing') }}</span>
      <strong>{{ title }} · {{ t(`collaboration.areas.${area.id}`) }}</strong>
    </div>
    <BaseSegmented
      :model-value="level === 'custom' ? '' : level"
      :options="levelOptions"
      :disabled="disabled"
      :aria-label="t(`collaboration.areas.${area.id}`)"
      @update:model-value="setLevel($event as AccessLevel)"
    />
    <div
      v-if="hasAccess"
      class="access-inspector__scope"
    >
      <BaseSegmented
        size="sm"
        :model-value="scopeMode"
        :disabled="disabled"
        :aria-label="`${t(`collaboration.areas.${area.id}`)} — ${t('collaboration.scopes')}`"
        :options="[{ value: 'all', label: t('collaboration.scopeModes.all') }, { value: 'selected', label: t('collaboration.onlyIds') }]"
        @update:model-value="setMode($event as 'all' | 'selected')"
      />
      <BaseTextField
        v-if="scopeMode === 'selected'"
        :model-value="idsText"
        :aria-label="`${t(`collaboration.areas.${area.id}`)} — ${t('collaboration.ids')}`"
        :placeholder="t(area.id === 'automations' ? 'collaboration.automationIds' : 'collaboration.idsComma')"
        :disabled="disabled"
        maxlength="65536"
        @update:model-value="setIds"
      />
    </div>
    <label
      v-if="area.id === 'points' && level === 'manage'"
      class="access-inspector__limit"
    >
      {{ t('collaboration.pointsLimit') }}
      <input
        type="number"
        min="0"
        step="any"
        :value="grant.maxPointsDelta ?? ''"
        :disabled="disabled"
        @input="setLimit"
      >
    </label>
    <p
      v-if="hasAccess && scopeMode === 'selected' && !scope?.length"
      class="access-inspector__hint access-inspector__hint--warning"
    >
      {{ t('collaboration.noIdsHint') }}
    </p>
    <p
      v-if="area.write.length"
      class="access-inspector__hint"
    >
      {{ t(`collaboration.manageHints.${area.id}`) }}
    </p>
    <p
      v-if="level === 'custom'"
      class="access-inspector__hint"
    >
      {{ t('collaboration.customHint') }}
    </p>
    <p
      v-if="lockedSomewhere"
      class="access-inspector__hint"
    >
      {{ t('collaboration.lockedHint') }}
    </p>
    <div
      v-if="saveable"
      class="access-inspector__actions"
    >
      <BaseButton
        :disabled="disabled || !dirty"
        @click="emit('discard')"
      >
        {{ t('collaboration.discard') }}
      </BaseButton>
      <BaseButton
        variant="primary"
        :disabled="disabled || !dirty"
        @click="emit('apply')"
      >
        {{ t('collaboration.grantSave') }}
      </BaseButton>
    </div>
  </section>
</template>
