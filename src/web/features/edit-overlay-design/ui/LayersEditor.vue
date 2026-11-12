<script setup lang="ts">
import { computed, markRaw, ref, shallowRef, useId, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { WidgetLayer, WidgetTemplateToken } from '../../../../widgets/sdk/template.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseColorField from '@/shared/ui/color-field/BaseColorField.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconChevronUp from '@/shared/ui/icon/icons/IconChevronUp.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import BaseList from '@/shared/ui/list/BaseList.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';
import BaseSlider from '@/shared/ui/slider/BaseSlider.vue';
import BaseTextArea from '@/shared/ui/text-area/BaseTextArea.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import { LAYER_NAME_MAX_LENGTH, TEXT_MAX_LENGTH, type DesignEditor, type Placement } from '../model/use-design-editor.ts';

const props = defineProps<{
  editor: DesignEditor;
}>();

type Side = NonNullable<WidgetLayer['placement']>;

const { t } = useI18n();
const id = useId();
const selected = ref<string | null>(null);
const addOpen = ref(false);
const addAnchor = shallowRef<HTMLElement | null>(null);

const layers = computed(() => props.editor.layers.value);
const index = computed(() => layers.value.findIndex((layer) => layer.id === selected.value));
const layer = computed(() => layers.value[index.value] ?? null);
const tokens = computed(() => props.editor.schema.value.tokens);
const kinds = computed<WidgetLayer['kind'][]>(() =>
  props.editor.template.value.id === 'gift' ? ['text', 'avatar', 'art'] : ['text', 'avatar'],
);
const addMenu = computed<MenuEntry[]>(() =>
  kinds.value.map((kind) => ({
    id: kind,
    label: t(`overlays.editor.layerKinds.${kind}`),
    children: [
      { id: `${kind}:top`, label: t('overlays.editor.addAtTop') },
      { id: `${kind}:bottom`, label: t('overlays.editor.addAtBottom') },
    ],
  })),
);
const sides = computed(() =>
  (['left', 'right', 'top', 'bottom'] as const).map((side) => ({ value: side, label: t(`overlays.editor.placements.${side}`) })),
);
const fontSize = computed(() => layer.value?.fontSize ?? (layer.value?.field === 'name' ? 27 : 15));
const fontWeight = computed(() => layer.value?.fontWeight ?? (layer.value?.field === 'name' ? 800 : 400));
const mediaSize = computed(() => layer.value?.size ?? (layer.value?.kind === 'art' ? 88 : props.editor.defaults.value.avatarSize));

watch(layers, (next) => {
  if (selected.value && !next.some((entry) => entry.id === selected.value)) selected.value = null;
});

function edit(key: string, patch: Partial<WidgetLayer>): void {
  if (layer.value) props.editor.editLayer(layer.value.id, key, patch);
}

function appendToken(token: WidgetTemplateToken): void {
  const text = `${layer.value?.text ?? ''}{{${token}}}`;
  if (text.length <= TEXT_MAX_LENGTH) edit('text', { text });
}

function openAdd(event: MouseEvent): void {
  addAnchor.value = markRaw(event.currentTarget as HTMLElement);
  addOpen.value = true;
}

function add(action: MenuAction): void {
  const [kind, placement] = action.id.split(':') as [WidgetLayer['kind'], Placement | undefined];
  if (placement) selected.value = props.editor.addLayer(kind, placement);
}
</script>

<template>
  <div class="layers-editor">
    <div class="layers-editor__toolbar">
      <BaseButton
        size="sm"
        aria-haspopup="menu"
        @click="openAdd"
      >
        <template #icon>
          <IconPlus />
        </template>
        {{ t('overlays.editor.addLayer') }}
      </BaseButton>
      <span class="layers-editor__spacer" />
      <BaseIconButton
        :label="t('overlays.editor.moveUp')"
        size="sm"
        :disabled="index <= 0"
        @click="layer && editor.moveLayer(layer.id, -1)"
      >
        <IconChevronUp />
      </BaseIconButton>
      <BaseIconButton
        :label="t('overlays.editor.moveDown')"
        size="sm"
        :disabled="index < 0 || index >= layers.length - 1"
        @click="layer && editor.moveLayer(layer.id, 1)"
      >
        <IconChevronDown />
      </BaseIconButton>
      <BaseIconButton
        :label="t('overlays.editor.removeLayer')"
        size="sm"
        :disabled="!layer"
        @click="layer && editor.removeLayer(layer.id)"
      >
        <IconTrash />
      </BaseIconButton>
    </div>
    <BaseList
      v-model:selected="selected"
      class="layers-editor__list"
      :items="layers"
      :item-key="(entry) => entry.id"
      :aria-label="t('overlays.editor.sections.layers')"
    >
      <template #default="{ item }">
        <span class="layers-editor__number">{{ layers.indexOf(item) + 1 }}</span>
        <span class="layers-editor__name">{{ item.name }}</span>
        <span class="layers-editor__kind">{{ t(`overlays.editor.layerKinds.${item.kind}`) }}</span>
      </template>
      <template #empty>
        {{ t('overlays.editor.noLayers') }}
      </template>
    </BaseList>
    <BaseFormGrid
      v-if="layer"
      class="layers-editor__details"
    >
      <BaseFormRow
        :for="`${id}-name`"
        :label="t('overlays.editor.layerName')"
      >
        <BaseTextField
          :id="`${id}-name`"
          :model-value="layer.name"
          :maxlength="LAYER_NAME_MAX_LENGTH"
          @update:model-value="edit('name', { name: $event })"
        />
      </BaseFormRow>
      <template v-if="layer.kind === 'text'">
        <BaseFormRow
          :for="`${id}-text`"
          :label="t('overlays.editor.layerContent')"
        >
          <BaseTextArea
            :id="`${id}-text`"
            :model-value="layer.text ?? ''"
            :rows="3"
            :max-length="TEXT_MAX_LENGTH"
            @update:model-value="edit('text', { text: $event })"
          />
          <span class="layers-editor__tokens">
            <BaseButton
              v-for="token in tokens"
              :key="token"
              size="sm"
              @click="appendToken(token)"
            >
              {{ t(`overlays.editor.tokens.${token}`) }}
            </BaseButton>
          </span>
        </BaseFormRow>
        <BaseFormRow
          :for="`${id}-color`"
          :label="t('overlays.editor.color')"
        >
          <BaseColorField
            :id="`${id}-color`"
            :model-value="layer.color ?? editor.defaults.value.textColor"
            @update:model-value="edit('color', { color: $event })"
          />
        </BaseFormRow>
        <BaseFormRow
          :for="`${id}-size`"
          :label="t('overlays.editor.fontSize')"
        >
          <span class="layers-editor__range">
            <BaseSlider
              :id="`${id}-size`"
              :model-value="fontSize"
              :min="8"
              :max="96"
              @update:model-value="edit('fontSize', { fontSize: $event })"
            />
            <output>{{ fontSize }} px</output>
          </span>
        </BaseFormRow>
        <BaseFormRow
          :for="`${id}-weight`"
          :label="t('overlays.editor.weight')"
        >
          <span class="layers-editor__range">
            <BaseSlider
              :id="`${id}-weight`"
              :model-value="fontWeight"
              :min="100"
              :max="900"
              :step="100"
              @update:model-value="edit('fontWeight', { fontWeight: $event })"
            />
            <output>{{ fontWeight }}</output>
          </span>
        </BaseFormRow>
      </template>
      <template v-else>
        <BaseFormRow
          :for="`${id}-side`"
          :label="t('overlays.editor.imageSide')"
        >
          <BaseSegmented
            :id="`${id}-side`"
            :model-value="layer.placement ?? 'left'"
            :options="sides"
            size="sm"
            @update:model-value="(side: Side) => edit('placement', { placement: side === 'left' ? undefined : side })"
          />
        </BaseFormRow>
        <BaseFormRow
          :for="`${id}-media-size`"
          :label="t('overlays.editor.size')"
        >
          <span class="layers-editor__range">
            <BaseSlider
              :id="`${id}-media-size`"
              :model-value="mediaSize"
              :min="16"
              :max="160"
              @update:model-value="edit('size', { size: $event })"
            />
            <output>{{ mediaSize }} px</output>
          </span>
        </BaseFormRow>
      </template>
    </BaseFormGrid>
    <BaseMenu
      v-model:open="addOpen"
      :items="addMenu"
      :anchor="addAnchor"
      @select="add"
    />
  </div>
</template>

<style scoped>
.layers-editor {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.layers-editor__hint {
  margin: 0;
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
}

.layers-editor__toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-1);
}

.layers-editor__spacer {
  flex: 1;
}

.layers-editor__list {
  max-height: 220px;
  border: var(--border-width) solid var(--color-border-subtle);
}

.layers-editor__number {
  min-width: 18px;
  color: var(--color-text-subtle);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}

.layers-editor__name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
}

.layers-editor__kind {
  opacity: 0.7;
  font-size: var(--font-size-sm);
}

.layers-editor__tokens {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
  margin-top: var(--space-2);
}

.layers-editor__range {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}

.layers-editor__range > :first-child {
  flex: 1;
}

.layers-editor__range output {
  min-width: 48px;
  color: var(--color-text-muted);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  text-align: right;
}
</style>
