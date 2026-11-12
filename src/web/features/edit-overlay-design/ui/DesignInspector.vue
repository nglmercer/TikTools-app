<script setup lang="ts">
import { computed, markRaw, ref, shallowRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { WidgetEditorControl, WidgetEditorSection } from '../../../../widgets/sdk/template.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import IconRedo from '@/shared/ui/icon/icons/IconRedo.vue';
import IconMoreVertical from '@/shared/ui/icon/icons/IconMoreVertical.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import {
  isStructuralControl,
  scalarControls,
  type ControlLabels,
  type ScalarControl,
  type ScalarControlId,
} from '../lib/design-controls.ts';
import type { DesignEditor } from '../model/use-design-editor.ts';
import DesignControlRow from './DesignControlRow.vue';
import LayersEditor from './LayersEditor.vue';
import TextLinesEditor from './TextLinesEditor.vue';

type SectionId = WidgetEditorSection['id'];

type Block =
  | { type: 'grid'; key: string; controls: { id: ScalarControlId; control: ScalarControl }[] }
  | { type: 'lines' | 'add' | 'layers'; key: string };

const props = withDefaults(
  defineProps<{
    editor: DesignEditor;
    saving?: boolean;
    status?: string;
    error?: string | null;
  }>(),
  { saving: false, status: '', error: null },
);

const emit = defineEmits<{ save: []; discard: [] }>();

const { t } = useI18n();
const section = ref<SectionId>('layers');
const optionsOpen = ref(false);
const optionsAnchor = shallowRef<HTMLElement | null>(null);

const controlIds: readonly ScalarControlId[] = [
  'textColor', 'background', 'accent', 'borderColor', 'radius', 'borderWidth', 'opacity', 'padding', 'gap', 'width', 'shadow',
  'autoWidth', 'align', 'avatarVisible', 'avatarSize', 'avatarRadius', 'avatarBorderColor', 'avatarBorderWidth', 'headingVisible',
  'headingColor', 'headingFontSize', 'headingWeight', 'headingSpacing',
];

const controls = computed(() => {
  const labels = Object.fromEntries(controlIds.map((id) => [id, t(`overlays.editor.controls.${id}`)])) as ControlLabels;
  return scalarControls(labels);
});

const sections = computed(() => props.editor.sections.value);
const tabs = computed(() => sections.value.map((entry) => ({ value: entry.id, label: t(`overlays.editor.sections.${entry.id}`) })));
const current = computed(() => sections.value.find((entry) => entry.id === section.value) ?? sections.value[0]);

function blocksFor(groupControls: readonly WidgetEditorControl[], prefix: string): Block[] {
  const blocks: Block[] = [];
  for (const control of groupControls) {
    if (isStructuralControl(control)) {
      const type = control === 'layers' ? 'layers' : control === 'textFields' ? 'lines' : 'add';
      blocks.push({ type, key: `${prefix}-${control}` });
      continue;
    }
    const last = blocks.at(-1);
    const entry = { id: control, control: controls.value[control] };
    if (last?.type === 'grid') last.controls.push(entry);
    else blocks.push({ type: 'grid', key: `${prefix}-${control}`, controls: [entry] });
  }
  return blocks;
}

const groups = computed(() =>
  (current.value?.groups ?? []).map((group, index) => ({
    key: `${current.value?.id}-${index}`,
    title: group.title ? t(`overlays.editor.groups.${group.title}`) : '',
    blocks: blocksFor(group.controls, `${current.value?.id}-${index}`),
  })),
);

const options = computed<MenuEntry[]>(() => [
  { id: 'section', label: t('overlays.editor.resetSection') },
  { id: 'design', label: t('overlays.editor.resetDesign') },
]);

watch(sections, (next) => {
  if (!next.some((entry) => entry.id === section.value)) section.value = next[0]?.id ?? 'text';
}, { immediate: true });

function openOptions(event: MouseEvent): void {
  optionsAnchor.value = markRaw(event.currentTarget as HTMLElement);
  optionsOpen.value = true;
}

function onOption(action: MenuAction): void {
  if (action.id === 'section' && current.value) props.editor.resetSection(current.value.id, controls.value);
  else if (action.id === 'design') props.editor.resetDesign();
}

function onKeydown(event: KeyboardEvent): void {
  if (!(event.ctrlKey || event.metaKey) || props.saving) return;
  const key = event.key.toLowerCase();
  const target = event.target as HTMLElement | null;
  if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') && key === 'z') return;
  if (key === 'z') {
    event.preventDefault();
    if (event.shiftKey) props.editor.redo();
    else props.editor.undo();
  } else if (key === 'y') {
    event.preventDefault();
    props.editor.redo();
  }
}
</script>

<template>
  <BasePanel
    class="design-inspector"
    header-layout="inline"
    :title="t('overlays.editor.title')"
    flush
    @keydown="onKeydown"
  >
    <template #actions>
      <BaseIconButton
        :label="t('overlays.editor.undo')"
        size="sm"
        :disabled="!editor.canUndo.value || saving"
        @click="editor.undo()"
      >
        <IconUndo />
      </BaseIconButton>
      <BaseIconButton
        :label="t('overlays.editor.redo')"
        size="sm"
        :disabled="!editor.canRedo.value || saving"
        @click="editor.redo()"
      >
        <IconRedo />
      </BaseIconButton>
      <BaseIconButton
        :label="t('overlays.editor.options')"
        size="sm"
        aria-haspopup="menu"
        :disabled="saving"
        @click="openOptions"
      >
        <IconMoreVertical />
      </BaseIconButton>
    </template>
    <BaseTabs
      v-model="section"
      class="design-inspector__tabs"
      :tabs="tabs"
    >
      <fieldset
        class="design-inspector__body"
        :disabled="saving"
      >
        <template
          v-for="group in groups"
          :key="group.key"
        >
          <component
            :is="group.title ? BaseGroupBox : 'div'"
            v-bind="group.title ? { title: group.title } : {}"
            class="design-inspector__group"
          >
            <template
              v-for="block in group.blocks"
              :key="block.key"
            >
              <BaseFormGrid v-if="block.type === 'grid'">
                <DesignControlRow
                  v-for="entry in block.controls"
                  :key="entry.id"
                  :control="entry.control"
                  :design="editor.draft.value"
                  :defaults="editor.defaults.value"
                  :disabled="saving"
                  @update="editor.setScalar(entry.control, entry.id, $event)"
                />
              </BaseFormGrid>
              <LayersEditor
                v-else-if="block.type === 'layers'"
                :editor="editor"
              />
              <TextLinesEditor
                v-else
                :editor="editor"
                :show-lines="block.type === 'lines'"
                :show-add="block.type === 'add'"
              />
            </template>
          </component>
        </template>
      </fieldset>
    </BaseTabs>
    <template #footer>
      <span
        class="design-inspector__status"
        :class="{ 'design-inspector__status--error': error }"
        role="status"
      >{{ error || status }}</span>
      <BaseButton
        size="sm"
        :disabled="!editor.dirty.value || saving"
        @click="emit('discard')"
      >
        {{ t('overlays.editor.discard') }}
      </BaseButton>
      <BaseButton
        size="sm"
        variant="primary"
        :disabled="!editor.dirty.value || saving"
        @click="emit('save')"
      >
        {{ saving ? t('overlays.editor.saving') : t('overlays.editor.save') }}
      </BaseButton>
    </template>
    <BaseMenu
      v-model:open="optionsOpen"
      :items="options"
      :anchor="optionsAnchor"
      placement="bottom-end"
      @select="onOption"
    />
  </BasePanel>
</template>

<style scoped>
.design-inspector {
  height: 100%;
}

.design-inspector__tabs {
  height: 100%;
}

.design-inspector__body {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
  height: 100%;
  min-width: 0;
  margin: 0;
  padding: var(--space-4);
  overflow-y: auto;
  border: 0;
}

.design-inspector__group {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.design-inspector__status {
  flex: 1;
  overflow: hidden;
  color: var(--color-success);
  font-size: var(--font-size-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.design-inspector__status--error {
  color: var(--color-danger-hover);
}
</style>
