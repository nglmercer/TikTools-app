<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { WidgetLayer, WidgetLayerSeed } from '../../../../widgets/sdk/template.ts';
import type { WidgetKind } from '../../../../widgets/sdk/templates.ts';
import { cloneDesign } from '@/entities/overlay/lib/editor-history.ts';
import { gatewayTone, isOverlayKind, overlayKinds } from '@/entities/overlay/lib/overlay-host.ts';
import { useOverlaysStore } from '@/entities/overlay/model/overlays-store.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { useDesignEditor } from '@/features/edit-overlay-design/model/use-design-editor.ts';
import DesignInspector from '@/features/edit-overlay-design/ui/DesignInspector.vue';
import { loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import IconGlobe from '@/shared/ui/icon/icons/IconGlobe.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import BaseList from '@/shared/ui/list/BaseList.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import OverlayCloudPanel from '@/widgets/overlay-cloud/ui/OverlayCloudPanel.vue';
import OverlayPreviewPanel from '@/widgets/overlay-preview/ui/OverlayPreviewPanel.vue';

const { t } = useI18n();
const cloudOpen = ref(false);
const capabilities = useCapabilityStore();
// Cloud overlays and image packages appear only when the host announces them.
const cloudAvailable = computed(
  () => capabilities.supports('bundles.overlays.create') === true || capabilities.supports('bundles.overlays.list') === true || capabilities.supports('bundles.packages.preflight') === true,
);
const store = useOverlaysStore();
const { status, statusError, refreshing, designs, designsError } = storeToRefs(store);

const savedKind = loadViewState().widgets?.selectedKind;
const kind = ref<WidgetKind>(isOverlayKind(savedKind) ? savedKind : 'follow');
const pendingKind = ref<WidgetKind | null>(null);
const confirmOpen = ref(false);
const saving = ref(false);
const saveError = ref<string | null>(null);
const saved = ref(false);

const source = computed(() => designs.value[kind.value] ?? {});
const editor = useDesignEditor(kind, source, {
  layer: (seed: WidgetLayerSeed) =>
    seed.field ? t(`overlays.editor.fields.${seed.field}`) : t(`overlays.editor.layerKinds.${seed.kind}`),
  newLayer: (layerKind: WidgetLayer['kind']) => t(`overlays.editor.layerKinds.${layerKind}`),
  newText: () => t('overlays.editor.newText'),
});

const items = computed(() => overlayKinds.map((value) => ({ kind: value, name: t(`overlays.kinds.${value}.name`) })));
const selected = computed<WidgetKind | null>({
  get: () => kind.value,
  set: (next) => {
    if (next) requestKind(next);
  },
});
const tone = computed(() => gatewayTone(status.value?.state ?? null));
const gatewayText = computed(() => {
  const current = status.value;
  if (!current) return t('overlays.gateway.states.checking');
  return t(`overlays.gateway.states.${current.state}`, { port: current.port });
});
const gatewayHint = computed(() => statusError.value ?? status.value?.error ?? null);
const kindName = computed(() => t(`overlays.kinds.${kind.value}.name`));

function requestKind(next: WidgetKind): void {
  if (next === kind.value) return;
  if (editor.dirty.value) {
    pendingKind.value = next;
    confirmOpen.value = true;
    return;
  }
  kind.value = next;
}

function confirmDiscard(): void {
  confirmOpen.value = false;
  if (pendingKind.value) kind.value = pendingKind.value;
  else editor.load();
  pendingKind.value = null;
}

function onConfirmToggle(open: boolean): void {
  if (!open) pendingKind.value = null;
}

async function save(): Promise<void> {
  saving.value = true;
  saveError.value = null;
  try {
    await store.saveDesign(kind.value, cloneDesign(editor.draft.value));
    editor.load();
    saved.value = true;
  } catch (failure) {
    saveError.value = errorMessage(failure);
  } finally {
    saving.value = false;
  }
}

watch(kind, (value) => {
  saveViewState('widgets', { selectedKind: value });
  saved.value = false;
  saveError.value = null;
  editor.load();
});

watch(source, () => {
  if (!editor.dirty.value) editor.load();
});

watch(editor.dirty, (dirty) => {
  if (dirty) saved.value = false;
});

onMounted(() => {
  void store.refresh();
  void store.loadDesigns();
});

onBeforeUnmount(() => store.dispose());
</script>

<template>
  <div class="overlays-page">
    <header class="overlays-page__toolbar">
      <div class="overlays-page__heading">
        <IconLayers /><h1>{{ t('navigation.overlays') }}</h1>
      </div>
      <div class="overlays-page__actions">
        <BaseButton
          v-if="cloudAvailable"
          :aria-label="t('overlays.cloud')"
          aria-haspopup="dialog"
          variant="primary"
          @click="cloudOpen = true"
        >
          <template #icon>
            <IconGlobe />
          </template>{{ t('overlayLibrary.browse') }}
        </BaseButton>
      </div>
      <span
        class="overlays-page__gateway"
        :class="`overlays-page__gateway--${tone}`"
        role="status"
        :title="gatewayHint || undefined"
        :aria-description="gatewayHint || undefined"
      >
        <span
          class="overlays-page__dot"
          aria-hidden="true"
        />
        <span class="overlays-page__gateway-text">{{ gatewayText }}</span>
      </span>
      <BaseIconButton
        :label="t('overlays.gateway.refresh')"
        size="sm"
        :disabled="refreshing"
        @click="store.refresh()"
      >
        <IconRefresh />
      </BaseIconButton>
    </header>

    <p
      v-if="designsError"
      class="overlays-page__error"
      role="alert"
    >
      <span>{{ t('overlays.editor.loadError', { error: designsError }) }}</span>
      <BaseButton
        size="sm"
        @click="store.loadDesigns()"
      >
        {{ t('overlays.editor.retry') }}
      </BaseButton>
    </p>

    <OverlayCloudPanel
      v-if="cloudAvailable"
      v-model:open="cloudOpen"
      @select="requestKind($event); cloudOpen = false"
    />
    <BaseComboBox
      :model-value="kind"
      class="overlays-page__mobile-kind"
      :options="items.map(item => ({ value: item.kind, label: item.name }))"
      :aria-label="t('overlays.widgets')"
      @update:model-value="requestKind($event as WidgetKind)"
    />
    <div class="overlays-page__workspace">
      <BasePanel
        class="overlays-page__list"
        :title="t('overlays.widgets')"
        flush
      >
        <BaseList
          v-model:selected="selected"
          class="overlays-page__kinds"
          :items="items"
          :item-key="(item) => item.kind"
          :aria-label="t('overlays.widgets')"
        >
          <template #default="{ item }">
            <IconLayers class="overlays-page__kind-icon" />
            <span class="overlays-page__kind-name">{{ item.name }}</span>
            <span
              v-if="item.kind === kind && editor.dirty.value"
              class="overlays-page__dirty"
              aria-hidden="true"
            >●</span>
          </template>
        </BaseList>
      </BasePanel>
      <OverlayPreviewPanel
        class="overlays-page__preview"
        :kind="kind"
        :design="editor.draft.value"
      />
      <DesignInspector
        class="overlays-page__inspector"
        :editor="editor"
        :saving="saving"
        :error="saveError"
        :status="saved ? t('overlays.editor.saved') : ''"
        @save="save"
        @discard="confirmOpen = true"
      />
    </div>

    <BaseConfirmDialog
      v-model:open="confirmOpen"
      :title="t('overlays.editor.discardTitle')"
      :message="t('overlays.editor.discardMessage', { name: kindName })"
      :confirm-label="t('overlays.editor.discardConfirm')"
      danger
      @confirm="confirmDiscard"
      @update:open="onConfirmToggle"
    />
  </div>
</template>

<style scoped>
.overlays-page__heading { display:flex; align-items:center; gap:var(--space-4); }
.overlays-page__heading h1 { margin:0; font-size:var(--font-size-lg); font-weight:var(--font-weight-semibold); }
.overlays-page__actions { display:flex; gap:var(--space-4); margin-left:auto; }
.overlays-page__toolbar { flex-wrap:wrap; padding-bottom:var(--space-4); border-bottom:1px solid var(--color-border-subtle); }
.overlays-page__gateway { flex-basis:auto; }
.overlays-page__workspace > * { border-radius:var(--radius-md); overflow:hidden; }
.overlays-page__list :deep(.base-list__item) { min-height:40px; }

.overlays-page {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
  min-height: 0;
  padding: var(--space-4);
}



.overlays-page__toolbar {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
}



.overlays-page__gateway {
  display: inline-flex;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
  color: var(--color-text-muted);
  white-space: nowrap;
}

.overlays-page__gateway-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.overlays-page__gateway-title {
  color: var(--color-text);
  font-weight: var(--font-weight-semibold);
}

.overlays-page__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-text-subtle);
}

.overlays-page__gateway--ok .overlays-page__dot {
  background: var(--color-success);
}

.overlays-page__gateway--busy .overlays-page__dot {
  background: var(--color-warning);
}

.overlays-page__gateway--error .overlays-page__dot {
  background: var(--color-danger);
}

.overlays-page__hint {
  overflow: hidden;
  color: var(--color-warning);
  font-size: var(--font-size-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.overlays-page__error {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-3);
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-danger);
  border-radius: var(--radius-sm);
  color: var(--color-danger-hover);
}

.overlays-page__error > span {
  flex: 1;
}

.overlays-page__workspace {
  display: grid;
  flex: 1;
  grid-template-columns: 200px minmax(0, 1fr) 320px;
  gap: var(--space-4);
  min-height: 0;
}

.overlays-page__kinds {
  height: 100%;
}

.overlays-page__kind-icon {
  flex: none;
  opacity: 0.75;
}

.overlays-page__kind-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
}

.overlays-page__dirty {
  color: var(--color-warning);
  font-size: 8px;
}

.overlays-page__description {
  margin: 0;
  padding: var(--space-2);
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
  white-space: normal;
}

@media (width < 1200px) {
  .overlays-page__workspace {
    grid-template-rows: minmax(260px, 1fr) minmax(0, 1fr);
    grid-template-columns: 200px minmax(0, 1fr);
  }

  .overlays-page__list {
    grid-row: span 2;
  }
}

.overlays-page__mobile-kind { display:none; }
@media (width < 800px) {
  .overlays-page {
    padding: var(--space-3);
    overflow-x: hidden;
    overflow-y: auto;
  }

  /* Toolbar wraps: the gateway status shrinks and the hint gets its own line. */
  .overlays-page__toolbar {
    flex-wrap: wrap;
    row-gap: var(--space-2);
  }

  .overlays-page__gateway {
    flex: 1;
  }

  .overlays-page__hint {
    flex-basis: 100%;
    white-space: normal;
  }

  .overlays-page__workspace {
    flex: none;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: auto auto;
  }

  .overlays-page__list { display:none; }
  .overlays-page__mobile-kind { display:flex; flex:none; }
  .overlays-page__heading { display:none; }
  .overlays-page__actions { margin-left:0; }
  .overlays-page__preview { min-height:300px; }
  .overlays-page__inspector { height:auto; min-height:360px; }
  .overlays-page__gateway { flex-basis:calc(100% - 60px); order:2; }
  .overlays-page__toolbar > :deep(.base-icon-button) { order:3; }
}
</style>
