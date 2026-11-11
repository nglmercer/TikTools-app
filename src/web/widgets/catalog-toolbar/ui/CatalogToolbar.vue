<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, useTemplateRef } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  activeFilterCount,
  DEFAULT_CATALOG_FILTERS,
  type CatalogFilters,
  type CatalogTypeFilter,
  type CatalogView,
} from '@/entities/catalog/model/catalog-query.ts';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

const props = defineProps<{ modelValue: CatalogFilters; signedIn: boolean }>();
const emit = defineEmits<{
  'update:modelValue': [value: CatalogFilters];
  action: [id: 'manageProcessors' | 'manageInstalled' | 'installPackage'];
  signIn: [];
}>();

const { t } = useI18n();
const root = useTemplateRef<HTMLElement>('root');

const count = computed(() => activeFilterCount(props.modelValue));
const views = computed<{ value: CatalogView; label: string }[]>(() => [
  { value: 'all', label: t('catalogToolbar.all') },
  { value: 'installed', label: t('catalogToolbar.installed') },
  { value: 'mine', label: t('catalogToolbar.mine') },
]);
const types = computed<{ value: CatalogTypeFilter; label: string }[]>(() => [
  { value: 'all', label: t('catalogToolbar.all') },
  { value: 'plugin', label: t('catalogToolbar.plugins') },
  { value: 'template', label: t('catalogToolbar.templates') },
  { value: 'profile', label: t('catalogToolbar.profiles') },
]);

const search = computed({
  get: () => props.modelValue.text,
  set: (text: string) => patch({ text }),
});

function patch(next: Partial<CatalogFilters>): void {
  emit('update:modelValue', { ...props.modelValue, ...next });
}

function reset(): void {
  emit('update:modelValue', { ...DEFAULT_CATALOG_FILTERS, text: props.modelValue.text });
}

// Native details do not close on Escape or outside clicks; the toolbar does both.
function closeDisclosures(except?: Element | null): void {
  for (const element of root.value?.querySelectorAll('details[open]') ?? []) {
    if (element !== except && !(except && element.contains(except))) element.removeAttribute('open');
  }
}
function onPointerDown(event: Event): void {
  const target = event.target as Element | null;
  if (!target || !root.value?.contains(target)) closeDisclosures();
  else {
    const owner = target.closest('details');
    closeDisclosures(owner ?? null);
  }
}
function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    const open = root.value?.querySelector('details[open]');
    if (open) {
      open.removeAttribute('open');
      open.querySelector('summary')?.focus();
    }
  }
}

onMounted(() => {
  document.addEventListener('pointerdown', onPointerDown);
  document.addEventListener('keydown', onKeydown);
});
onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', onPointerDown);
  document.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <div
    ref="root"
    class="catalog-toolbar"
    role="toolbar"
    :aria-label="t('catalogToolbar.filters')"
  >
    <BaseTextField
      v-model="search"
      type="search"
      class="catalog-toolbar__search"
      :placeholder="t('catalogToolbar.search')"
      :aria-label="t('catalogToolbar.search')"
    />
    <details class="catalog-toolbar__disclosure">
      <summary class="catalog-toolbar__trigger">
        {{ t('catalogToolbar.filters') }}
        <span
          v-if="count > 0"
          class="catalog-toolbar__badge"
        >{{ count }}</span>
      </summary>
      <div class="catalog-toolbar__panel">
        <fieldset class="catalog-toolbar__group">
          <legend>{{ t('catalogToolbar.view') }}</legend>
          <label
            v-for="option in views"
            :key="option.value"
          >
            <input
              type="radio"
              name="catalog-view"
              :value="option.value"
              :checked="modelValue.view === option.value"
              @change="patch({ view: option.value })"
            >
            {{ option.label }}
          </label>
        </fieldset>
        <fieldset class="catalog-toolbar__group">
          <legend>{{ t('catalogToolbar.type') }}</legend>
          <label
            v-for="option in types"
            :key="option.value"
          >
            <input
              type="radio"
              name="catalog-type"
              :value="option.value"
              :checked="modelValue.type === option.value"
              @change="patch({ type: option.value })"
            >
            {{ option.label }}
          </label>
        </fieldset>
        <label class="catalog-toolbar__check">
          <input
            type="checkbox"
            :checked="modelValue.compatibleOnly"
            @change="patch({ compatibleOnly: ($event.target as HTMLInputElement).checked })"
          >
          {{ t('catalogToolbar.compatible') }}
        </label>
        <button
          type="button"
          class="catalog-toolbar__reset"
          :disabled="count === 0"
          @click="reset"
        >
          {{ t('catalogToolbar.reset') }}
        </button>
      </div>
    </details>
    <details class="catalog-toolbar__disclosure catalog-toolbar__more">
      <summary
        class="catalog-toolbar__trigger"
        :aria-label="t('catalogToolbar.more')"
      >
        ⋯
      </summary>
      <div class="catalog-toolbar__panel">
        <button
          type="button"
          @click="emit('action', 'manageInstalled')"
        >
          {{ t('catalogToolbar.manageInstalled') }}
        </button>
        <button
          type="button"
          @click="emit('action', 'manageProcessors')"
        >
          {{ t('catalogToolbar.manageProcessors') }}
        </button>
        <button
          type="button"
          @click="emit('action', 'installPackage')"
        >
          {{ t('catalogToolbar.installPackage') }}
        </button>
      </div>
    </details>
    <button
      v-if="modelValue.view === 'mine' && !signedIn"
      type="button"
      class="catalog-toolbar__signin"
      @click="emit('signIn')"
    >
      {{ t('account.signIn') }}
    </button>
  </div>
</template>

<style scoped>
.catalog-toolbar { display:flex; align-items:center; flex-wrap:wrap; gap:var(--space-2); min-width:0; position:relative; }
.catalog-toolbar__search { flex:1 1 220px; min-width:0; }
.catalog-toolbar__disclosure { position:relative; }
.catalog-toolbar__trigger { cursor:pointer; list-style:none; display:inline-flex; align-items:center; gap:var(--space-2); min-height:40px; padding:0 var(--space-3); border-radius:var(--radius-md); border:1px solid var(--color-border); }
.catalog-toolbar__trigger::-webkit-details-marker { display:none; }
.catalog-toolbar__trigger:focus-visible { outline:2px solid var(--color-focus); outline-offset:2px; }
.catalog-toolbar__badge { min-width:20px; padding:0 6px; border-radius:999px; background:var(--color-accent); color:var(--color-on-accent); font-size:var(--font-size-xs); text-align:center; }
.catalog-toolbar__panel { position:absolute; inset-inline-end:0; top:calc(100% + 6px); z-index:10; display:grid; gap:var(--space-3); min-width:220px; padding:var(--space-3); border-radius:var(--radius-md); border:1px solid var(--color-border); background:var(--color-surface-raised); }
.catalog-toolbar__group { display:grid; gap:var(--space-1); border:0; padding:0; margin:0; }
.catalog-toolbar__group legend { font-size:var(--font-size-sm); color:var(--color-text-muted); }
.catalog-toolbar__group label, .catalog-toolbar__check { display:flex; align-items:center; gap:var(--space-2); min-height:32px; }
.catalog-toolbar__reset, .catalog-toolbar__signin { min-height:40px; padding:0 var(--space-3); border-radius:var(--radius-md); }
@media (width < 600px) { .catalog-toolbar__search { flex-basis:100%; } .catalog-toolbar__panel { inset-inline-start:0; inset-inline-end:auto; } }
</style>
