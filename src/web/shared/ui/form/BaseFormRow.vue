<script setup lang="ts">
import { computed, onMounted, onUpdated, ref, useId, useTemplateRef } from 'vue';
const props = defineProps<{
  label: string;
  for?: string;
  hint?: string;
  error?: string;
  required?: boolean;
  stacked?: boolean;
  group?: boolean;
}>();
const id = useId();
const inferredId = ref(`field-${id}`);
const controlId = computed(() => props.for || inferredId.value);
const hintId = `hint-${id}`;
const errorId = `error-${id}`;
const labelId = `label-${id}`;
const describedBy = computed(() => [props.hint ? hintId : '', props.error ? errorId : ''].filter(Boolean).join(' ') || undefined);
const control = useTemplateRef('control');
let ownedInvalid: HTMLElement | null = null;
let ownedRequired: HTMLElement | null = null;
// Preserve existing callers and their own descriptions; scoped slot attrs also
// support controls whose actual input is rendered outside this container.
function associate(): void {
  if (props.group) return;
  const fields = Array.from(control.value?.querySelectorAll<HTMLElement>('input,select,textarea,[role="combobox"],[role="switch"]') ?? []);
  const field = fields.find(item => item.id === controlId.value) ?? fields[0];
  if (!field) return;
  if (!field.id) field.id = controlId.value;
  else if (!props.for) inferredId.value = field.id;
  const ids = (field.getAttribute('aria-describedby') ?? '').split(' ').filter(value => value && value !== hintId && value !== errorId);
  if (describedBy.value) ids.push(...describedBy.value.split(' '));
  if (ids.length) field.setAttribute('aria-describedby', [...new Set(ids)].join(' '));
  else field.removeAttribute('aria-describedby');
  if (props.required) {
    if (field.getAttribute('aria-required') !== 'true') ownedRequired = field;
    field.setAttribute('aria-required', 'true');
  } else if (ownedRequired === field) { field.removeAttribute('aria-required'); ownedRequired = null; }
  if (props.error) {
    if (field.getAttribute('aria-invalid') !== 'true') ownedInvalid = field;
    field.setAttribute('aria-invalid', 'true');
  } else if (ownedInvalid === field) { field.removeAttribute('aria-invalid'); ownedInvalid = null; }
}
onMounted(associate);
onUpdated(associate);
</script>

<template>
  <div
    class="base-form-row"
    :class="{ 'base-form-row--stacked': stacked }"
  >
    <component
      :is="group ? 'div' : 'label'"
      :id="labelId"
      class="base-form-row__label"
      :for="group ? undefined : controlId"
    >
      {{ label }}<span
        v-if="required"
        aria-hidden="true"
      > *</span>
    </component>
    <div
      ref="control"
      class="base-form-row__control"
      :role="group ? 'group' : undefined"
      :aria-labelledby="group ? labelId : undefined"
      :aria-describedby="group ? describedBy : undefined"
    >
      <slot :control-attrs="{ id: controlId, 'aria-describedby': describedBy, 'aria-invalid': error ? true : undefined, 'aria-required': required || undefined }" />
      <p
        v-if="hint"
        :id="hintId"
        class="base-form-row__hint"
      >
        {{ hint }}
      </p>
      <p
        v-if="error"
        :id="errorId"
        class="base-form-row__error"
        role="alert"
      >
        {{ error }}
      </p>
    </div>
  </div>
</template>

<style scoped>
.base-form-row { display: contents; }
.base-form-row__label { justify-self: end; color: var(--color-text); text-align: right; }
.base-form-row__control { display: flex; flex-direction: column; gap: var(--space-2); min-width: 0; }
.base-form-row--stacked { display: grid; gap: var(--space-2); min-width: 0; }
.base-form-row--stacked > .base-form-row__label { justify-self: start; text-align: left; }
.base-form-row__hint,.base-form-row__error { margin: 0; font-size: var(--font-size-sm); line-height: var(--line-height-normal); overflow-wrap: anywhere; }
.base-form-row__hint { color: var(--color-text-muted); }
.base-form-row__error { color: var(--color-text-danger); }
@media (width < 600px) {
  .base-form-row__label { justify-self: start; margin-top: var(--space-3); color: var(--color-text-muted); text-align: left; }
}
</style>
