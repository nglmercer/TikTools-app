<script setup lang="ts">
import { computed, type Component } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseFileDrop from '@/shared/ui/file-drop/BaseFileDrop.vue';
import IconClose from '@/shared/ui/icon/icons/IconClose.vue';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import { MANUAL_IMPORT_TAB, type ImportDialogLabels, type ImportTab, type StagedImportItem } from './import-types.ts';

/**
 * One shell for every "bring something in" dialog: a dialog with a tab per source (Local,
 * Cloud, Manual…), where the manual tab is the file/paste drop zone with its staged items.
 * The shell owns the structure, tab semantics, limits, busy and error states; the owner
 * parses the content, fills `staged` and runs the import. Each tab body is the slot named
 * after the tab; a tab without a slot renders nothing but the manual one, which defaults to
 * the drop zone. With a single tab the tab bar is not drawn.
 */
const props = withDefaults(
  defineProps<{
    title: string;
    width?: number;
    tabs: readonly ImportTab[];
    /** Hides the tab bar, for a step that replaces the whole body (a form for the chosen item). */
    hideTabs?: boolean;
    /** Value of the tab shown by the drop zone. */
    manualTab?: string;
    /** File types the picker offers. */
    accept?: string;
    /** Largest accepted file or pasted text, in bytes. */
    maxBytes?: number;
    staged?: readonly StagedImportItem[];
    /** Where the staged items came from (a file name or the clipboard). */
    stagedSource?: string;
    /** Importing, or the owner is busy: the drop zone and the import button are disabled. */
    busy?: boolean;
    /** Problems to show; each one is announced. */
    errors?: readonly string[];
    /** Confirmation shown after a successful import. */
    success?: string;
    /** Text of the import button; defaults to "Import". */
    importLabel?: string;
    labels?: ImportDialogLabels;
  }>(),
  {
    width: 720,
    hideTabs: false,
    manualTab: MANUAL_IMPORT_TAB,
    accept: '.json,application/json',
    maxBytes: undefined,
    staged: () => [],
    stagedSource: '',
    busy: false,
    errors: () => [],
    success: '',
    importLabel: '',
    labels: () => ({}),
  },
);

const emit = defineEmits<{
  /** Text read from a file, a drop or the clipboard, with its source name. */
  content: [text: string, source: string];
  /** The file could not be read or exceeds the limit. */
  error: [message: string];
  /** The staged items were cleared. */
  clear: [];
  /** The user confirmed the import of the staged items. */
  import: [];
}>();

const open = defineModel<boolean>('open', { default: false });
const tab = defineModel<string>('tab', { required: true });

const { t } = useI18n();

function sizeText(bytes: number): string {
  return bytes >= 1024 * 1024 ? `${Math.round(bytes / (1024 * 1024))} MB` : `${Math.max(1, Math.round(bytes / 1024))} KB`;
}
const text = computed(() => ({
  dropTitle: props.labels.dropTitle ?? t('importDialog.dropTitle'),
  dropHint: props.labels.dropHint ?? t('importDialog.dropHint'),
  browse: props.labels.browse ?? t('importDialog.browse'),
  paste: props.labels.paste ?? t('importDialog.paste'),
  pasteBlocked: props.labels.pasteBlocked ?? t('importDialog.pasteBlocked'),
  clipboard: props.labels.clipboard ?? t('importDialog.clipboard'),
  unreadable: props.labels.unreadable ?? t('importDialog.unreadable'),
  tooLarge: props.labels.tooLarge ?? t('importDialog.tooLarge', { size: sizeText(props.maxBytes ?? 0) }),
  staged: props.labels.staged ?? t('importDialog.staged', { count: props.staged.length }),
  clear: props.labels.clear ?? t('importDialog.clear'),
  run: props.importLabel || props.labels.run || t('importDialog.run'),
  close: props.labels.close ?? t('importDialog.close'),
}));

const tabbed = computed(() => !props.hideTabs && props.tabs.length > 1);
const wrapper = computed<Component | string>(() => (tabbed.value ? BaseTabs : 'div'));
/** Props of the tab bar; a lone source is a plain wrapper that takes none. */
const wrapperProps = computed<Record<string, unknown>>(() =>
  tabbed.value
    ? {
        modelValue: tab.value,
        'onUpdate:modelValue': (value: string) => { tab.value = value; },
        tabs: props.tabs,
        label: t('importDialog.tabs'),
      }
    : {},
);
const manualActive = computed(() => tab.value === props.manualTab);
const canImport = computed(() => props.staged.length > 0 && !props.busy);
</script>

<template>
  <BaseDialog
    v-model:open="open"
    :title="title"
    :width="width"
  >
    <div
      class="base-import-dialog"
      :class="{ 'base-import-dialog--tabbed': tabbed }"
      :aria-busy="busy || undefined"
    >
      <!-- With one source (or a step that replaces the body) there is nothing to switch, so no tab bar. -->
      <component
        :is="wrapper"
        v-bind="wrapperProps"
      >
        <div class="base-import-dialog__body">
          <slot
            v-if="tab !== manualTab || $slots[manualTab]"
            :name="tab"
          />
          <template v-else>
            <slot name="manual-lead" />
            <BaseFileDrop
              :title="text.dropTitle"
              :hint="text.dropHint"
              :browse-label="text.browse"
              :paste-label="text.paste"
              :paste-blocked-label="text.pasteBlocked"
              :clipboard-label="text.clipboard"
              :unreadable-label="text.unreadable"
              :too-large-label="text.tooLarge"
              :accept="accept"
              :max-bytes="maxBytes"
              :disabled="busy"
              @content="(content, source) => emit('content', content, source)"
              @error="emit('error', $event)"
            />
            <section
              v-if="staged.length"
              class="base-import-dialog__staged"
              :aria-label="text.staged"
            >
              <header class="base-import-dialog__staged-head">
                <strong>{{ text.staged }}</strong>
                <span
                  v-if="stagedSource"
                  class="base-import-dialog__source"
                >{{ t('importDialog.from', { source: stagedSource }) }}</span>
                <BaseIconButton
                  class="base-import-dialog__clear"
                  :label="text.clear"
                  size="sm"
                  :disabled="busy"
                  @click="emit('clear')"
                >
                  <IconClose />
                </BaseIconButton>
              </header>
              <ul class="base-import-dialog__items">
                <li
                  v-for="item in staged"
                  :key="item.key"
                >
                  <slot
                    name="item"
                    :item="item"
                  >
                    {{ item.label }}
                    <span
                      v-if="item.detail"
                      class="base-import-dialog__detail"
                    >{{ item.detail }}</span>
                  </slot>
                </li>
              </ul>
              <slot name="staged-extra" />
            </section>
          </template>
        </div>
      </component>

      <p
        v-if="success"
        class="base-import-dialog__success"
        role="status"
      >
        {{ success }}
      </p>
      <p
        v-for="message in errors"
        :key="message"
        class="base-import-dialog__error"
        role="alert"
      >
        {{ message }}
      </p>
    </div>

    <template #footer>
      <slot
        name="footer"
        :tab="tab"
      >
        <BaseButton
          v-if="manualActive"
          variant="primary"
          :disabled="!canImport"
          @click="emit('import')"
        >
          {{ text.run }}
        </BaseButton>
        <BaseButton
          :disabled="busy && manualActive"
          @click="open = false"
        >
          {{ text.close }}
        </BaseButton>
      </slot>
    </template>
  </BaseDialog>
</template>

<style scoped>
.base-import-dialog {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
}

/* The dialog keeps one height across its sources, so switching tabs does not make it jump. */
.base-import-dialog--tabbed {
  min-height: min(50vh, 380px);
}

.base-import-dialog__body {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding-top: var(--space-4);
}

.base-import-dialog :deep(.base-tabs__panel) {
  min-width: 0;
}

.base-import-dialog__staged {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
}

.base-import-dialog__staged-head {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.base-import-dialog__source {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  color: var(--color-text-muted);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.base-import-dialog__clear {
  margin-left: auto;
}

.base-import-dialog__items {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
  padding-left: var(--space-5);
}

.base-import-dialog__detail {
  margin-left: var(--space-2);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.base-import-dialog__error,
.base-import-dialog__success {
  margin: 0;
  color: var(--color-danger-hover);
}

.base-import-dialog__success {
  color: var(--color-success);
}
</style>
