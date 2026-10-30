<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import WidgetHost from '../../../../widgets/sdk/WidgetHost.vue';
import type { WidgetStyle } from '../../../../widgets/sdk/template.ts';
import { widgetTemplates, type WidgetKind } from '../../../../widgets/sdk/templates.ts';
import { canCopySourceUrl, GATEWAY_DEFAULT_PORT, redactedSourceUrl } from '@/entities/overlay/lib/overlay-host.ts';
import { useOverlaysStore } from '@/entities/overlay/model/overlays-store.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconCopy from '@/shared/ui/icon/icons/IconCopy.vue';
import IconPlay from '@/shared/ui/icon/icons/IconPlay.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import RankingSourceOptions, { type RankingSourceSettings } from './RankingSourceOptions.vue';

const COPIED_MS = 2_000;

const props = defineProps<{
  kind: WidgetKind;
  design: WidgetStyle;
}>();

const { t } = useI18n();
const store = useOverlaysStore();
const replay = ref(0);
const copied = ref(false);
const copyError = ref<string | null>(null);
let copiedTimer: ReturnType<typeof setTimeout> | null = null;
const canvas = useTemplateRef('canvas');
const previewScale = ref(1);
let resizeObserver: ResizeObserver | null = null;

onMounted(() => {
  resizeObserver = new ResizeObserver(([entry]) => {
    if (entry) previewScale.value = Math.min(entry.contentRect.width / 640, entry.contentRect.height / 360);
  });
  if (canvas.value) resizeObserver.observe(canvas.value);
});

const template = computed(() => widgetTemplates[props.kind]);
const port = computed(() => store.status?.port ?? GATEWAY_DEFAULT_PORT);
const url = computed(() => redactedSourceUrl(port.value, props.kind));
const rankingSource = ref<RankingSourceSettings>({ rankingId: '', visualization: 'ranking', competitionId: '', showLabels: true, showPercentages: false, showLegend: true });
const copyReady = computed(() => canCopySourceUrl(store.status?.state ?? null) && !store.refreshing && (props.kind !== 'ranking' || !!rankingSource.value.rankingId));

function rankingOptions(): Record<string, unknown> {
  const source = rankingSource.value;
  if (source.visualization === 'roulette') return { rankingId: source.rankingId, visualization: 'roulette', ...(source.competitionId ? { competitionId: source.competitionId } : {}) };
  return { rankingId: source.rankingId, visualization: source.visualization, showLabels: source.showLabels, showPercentages: source.showPercentages, showLegend: source.showLegend };
}

function clearCopied(): void {
  if (copiedTimer !== null) clearTimeout(copiedTimer);
  copiedTimer = null;
  copied.value = false;
}

async function copy(): Promise<void> {
  clearCopied();
  copyError.value = null;
  const outcome = await store.copySourceUrl(props.kind, props.kind === 'ranking' ? rankingOptions() : {});
  if (!outcome.ok) {
    copyError.value = outcome.message || null;
    return;
  }
  copied.value = true;
  copiedTimer = setTimeout(clearCopied, COPIED_MS);
}

watch(
  () => props.kind,
  () => {
    clearCopied();
    copyError.value = null;
  },
);

onBeforeUnmount(() => {
  clearCopied();
  resizeObserver?.disconnect();
});
</script>

<template>
  <BasePanel
    class="overlay-preview"
    :title="t('overlays.preview.title')"
    :subtitle="t('overlays.preview.canvas')"
    flush
  >
    <template #actions>
      <BaseIconButton
        :label="t('overlays.preview.replay')"
        size="sm"
        @click="replay += 1"
      >
        <IconPlay />
      </BaseIconButton>
    </template>
    <div class="overlay-preview__workarea">
      <div
        ref="canvas"
        class="overlay-preview__canvas"
      >
        <div
          class="overlay-preview__stage"
          :style="{ transform: `translate(-50%, -50%) scale(${previewScale})` }"
        >
          <WidgetHost
            :key="kind"
            :template="template"
            mode="preview"
            :design="design"
            :replay-key="replay"
          />
        </div>
      </div>
    </div>
    <template #footer>
      <div class="overlay-preview__source">
        <RankingSourceOptions
          v-if="kind === 'ranking'"
          v-model="rankingSource"
        />
        <label
          class="overlay-preview__label"
          for="overlay-source-url"
        >{{ t('overlays.source.title') }}</label>
        <div class="overlay-preview__url">
          <BaseTextField
            id="overlay-source-url"
            :model-value="url"
            readonly
            size="sm"
            spellcheck="false"
          />
          <BaseButton
            size="sm"
            :disabled="!copyReady"
            @click="copy"
          >
            <template #icon>
              <IconCheck v-if="copied" />
              <IconCopy v-else />
            </template>
            {{ copied ? t('overlays.source.copied') : t('overlays.source.copy') }}
          </BaseButton>
        </div>
        <p
          v-if="copyError"
          class="overlay-preview__error"
          role="alert"
        >
          {{ copyError }}
        </p>
        <p class="overlay-preview__note">
          {{ t('overlays.source.note') }}
        </p>
      </div>
    </template>
  </BasePanel>
</template>

<style scoped>
.overlay-preview {
  height: 100%;
}

.overlay-preview__workarea {
  box-sizing: border-box;
  display: flex;
  height: 100%;
  align-items: center;
  justify-content: center;
  min-width: 0;
  min-height: 0;
  padding: var(--space-5);
  background: var(--color-bg-app);
}

.overlay-preview__canvas {
  position: relative;
  width: 100%;
  max-height: 100%;
  aspect-ratio: 16 / 9;
  overflow: hidden;
  border: var(--border-width) solid var(--color-border);
  background: repeating-conic-gradient(#20232b 0% 25%, #181b22 0% 50%) 0 0 / 24px 24px;
}

.overlay-preview__source {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
  padding: var(--space-2);
}

.overlay-preview__stage {
  position: absolute;
  top: 50%;
  left: 50%;
  width: 640px;
  height: 360px;
}

.overlay-preview__label {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.overlay-preview__url {
  display: flex;
  gap: var(--space-2);
}

.overlay-preview__url > :first-child {
  flex: 1;
  min-width: 0;
}

.overlay-preview__note,
.overlay-preview__error {
  margin: 0;
  font-size: var(--font-size-sm);
}

.overlay-preview__note {
  color: var(--color-text-subtle);
}

.overlay-preview__error {
  color: var(--color-danger-hover);
}
</style>
