<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { OVERLAY_FIELDS, OVERLAY_KINDS, type OverlayElement, type OverlayKind } from '../../../../shared/cloud-overlay-schema.ts';
import { useCloudOverlayStore } from '@/entities/overlay/model/cloud-overlay-store.ts';
import { useBundlesStore } from '@/entities/cloud-bundles/model/bundles-store.ts';
import CloudOverlayPreview from '@/entities/overlay/ui/CloudOverlayPreview.vue';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import IconInfo from '@/shared/ui/icon/icons/IconInfo.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';

const { t, te } = useI18n();
const overlay = useCloudOverlayStore();
const bundles = useBundlesStore();
const loadId = ref('');

const kindOptions = computed(() => OVERLAY_KINDS.map((kind) => ({ value: kind, label: t(`cloudOverlays.kinds.${kind}`) })));
const fieldOptions = computed(() => OVERLAY_FIELDS.map((field) => ({ value: field, label: field })));
const modeOptions = computed(() => [
  { value: 'static', label: t('cloudOverlays.static') },
  { value: 'field', label: t('cloudOverlays.liveField') },
]);

function problemLabel(code: string): string {
  const key = `cloudOverlays.problem.${code}`;
  return te(key) ? t(key) : code;
}

function num(value: string): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function setSource(element: OverlayElement, mode: string): void {
  if (element.type !== 'text') return;
  overlay.replaceElement(element.id, {
    ...element,
    source: mode === 'field' ? { kind: 'field', field: 'viewer.name' } : { kind: 'static', text: 'Text' },
  });
}

function setStaticText(element: OverlayElement, text: string): void {
  if (element.type !== 'text') return;
  overlay.replaceElement(element.id, { ...element, source: { kind: 'static', text } });
}

function setField(element: OverlayElement, field: string): void {
  if (element.type !== 'text') return;
  overlay.replaceElement(element.id, { ...element, source: { kind: 'field', field: field as (typeof OVERLAY_FIELDS)[number] } });
}

function setArt(element: OverlayElement, assetId: string): void {
  if (element.type !== 'art') return;
  overlay.replaceElement(element.id, { ...element, assetId });
}

function useLastImage(element: OverlayElement): void {
  if (bundles.asset) setArt(element, bundles.asset.id);
}

async function load(): Promise<void> {
  if (loadId.value.trim()) await overlay.load(loadId.value.trim());
}

function setKind(value: string): void {
  overlay.setKind(value as OverlayKind);
}
</script>

<template>
  <section class="cloud-overlays">
    <h3 class="cloud-overlays__title">
      {{ t('cloudOverlays.title') }}
      <!-- The explanation stays one hover away instead of a paragraph under the heading. -->
      <span
        v-tooltip="t('cloudOverlays.hint')"
        class="cloud-overlays__info"
        role="img"
        :aria-label="t('cloudOverlays.hint')"
      >
        <IconInfo :size="14" />
      </span>
    </h3>

    <form
      class="cloud-overlays__row"
      @submit.prevent="load"
    >
      <BaseTextField
        v-model="loadId"
        class="cloud-overlays__grow"
        :placeholder="t('cloudOverlays.overlayId')"
        :aria-label="t('cloudOverlays.overlayId')"
      />
      <BaseButton
        type="submit"
        :disabled="loadId.trim().length === 0 || overlay.saving"
      >
        {{ t('cloudOverlays.open') }}
      </BaseButton>
    </form>

    <div class="cloud-overlays__row">
      <BaseComboBox
        :model-value="overlay.draft.kind"
        :options="kindOptions"
        :aria-label="t('cloudOverlays.kind')"
        @update:model-value="setKind($event as string)"
      />
      <BaseTextField
        class="cloud-overlays__grow"
        :model-value="overlay.draft.title"
        :aria-label="t('cloudOverlays.name')"
        maxlength="120"
        @update:model-value="overlay.setTitle($event)"
      />
    </div>

    <div class="cloud-overlays__row">
      <BaseTextField
        inputmode="numeric"
        :model-value="String(overlay.draft.canvas.width)"
        :aria-label="t('cloudOverlays.canvasWidth')"
        @update:model-value="overlay.setCanvas(num($event), overlay.draft.canvas.height)"
      />
      <BaseTextField
        inputmode="numeric"
        :model-value="String(overlay.draft.canvas.height)"
        :aria-label="t('cloudOverlays.canvasHeight')"
        @update:model-value="overlay.setCanvas(overlay.draft.canvas.width, num($event))"
      />
      <BaseButton @click="overlay.add('text')">
        {{ t('cloudOverlays.addText') }}
      </BaseButton>
      <BaseButton @click="overlay.add('avatar')">
        {{ t('cloudOverlays.addAvatar') }}
      </BaseButton>
      <BaseButton @click="overlay.add('art')">
        {{ t('cloudOverlays.addArt') }}
      </BaseButton>
    </div>

    <ul class="cloud-overlays__list">
      <li
        v-for="element in overlay.draft.elements"
        :key="element.id"
        class="cloud-overlays__block"
      >
        <div class="cloud-overlays__row">
          <strong>{{ element.id }}</strong>
          <BaseTextField
            inputmode="numeric"
            :model-value="String(element.x)"
            :aria-label="t('cloudOverlays.x')"
            @update:model-value="overlay.patch(element.id, { x: num($event) })"
          />
          <BaseTextField
            inputmode="numeric"
            :model-value="String(element.y)"
            :aria-label="t('cloudOverlays.y')"
            @update:model-value="overlay.patch(element.id, { y: num($event) })"
          />
          <BaseTextField
            inputmode="numeric"
            :model-value="String(element.width)"
            :aria-label="t('cloudOverlays.width')"
            @update:model-value="overlay.patch(element.id, { width: num($event) })"
          />
          <BaseTextField
            inputmode="numeric"
            :model-value="String(element.height)"
            :aria-label="t('cloudOverlays.height')"
            @update:model-value="overlay.patch(element.id, { height: num($event) })"
          />
          <BaseButton
            size="sm"
            variant="danger"
            @click="overlay.remove(element.id)"
          >
            {{ t('cloudOverlays.remove') }}
          </BaseButton>
        </div>

        <div
          v-if="element.type === 'text'"
          class="cloud-overlays__row"
        >
          <BaseComboBox
            :model-value="element.source.kind"
            :options="modeOptions"
            :aria-label="t('cloudOverlays.textSource')"
            @update:model-value="setSource(element, $event as string)"
          />
          <BaseTextField
            v-if="element.source.kind === 'static'"
            class="cloud-overlays__grow"
            :model-value="element.source.text"
            :aria-label="t('cloudOverlays.text')"
            maxlength="200"
            @update:model-value="setStaticText(element, $event)"
          />
          <BaseComboBox
            v-else
            :model-value="element.source.field"
            :options="fieldOptions"
            :aria-label="t('cloudOverlays.field')"
            @update:model-value="setField(element, $event as string)"
          />
        </div>

        <div
          v-if="element.type === 'art'"
          class="cloud-overlays__row"
        >
          <BaseTextField
            class="cloud-overlays__grow"
            :model-value="element.assetId"
            :placeholder="t('cloudOverlays.assetId')"
            :aria-label="t('cloudOverlays.assetId')"
            @update:model-value="setArt(element, $event)"
          />
          <BaseButton
            size="sm"
            :disabled="!bundles.asset"
            @click="useLastImage(element)"
          >
            {{ t('cloudOverlays.useLastImage') }}
          </BaseButton>
        </div>
      </li>
    </ul>

    <CloudOverlayPreview :document="overlay.draft" />

    <ul
      v-if="overlay.problems.length"
      class="cloud-overlays__problems"
    >
      <li
        v-for="code in overlay.problems"
        :key="code"
      >
        {{ problemLabel(code) }}
      </li>
    </ul>

    <BaseAlert
      v-if="overlay.error"
      tone="error"
      :title="overlay.error === 'too-large' ? t('cloudOverlays.tooLarge') : overlay.error"
      compact
    />

    <div class="cloud-overlays__row">
      <BaseButton
        variant="primary"
        :disabled="overlay.problems.length > 0 || overlay.saving"
        @click="overlay.save()"
      >
        {{ overlay.overlayId ? t('cloudOverlays.saveRevision') : t('cloudOverlays.create') }}
      </BaseButton>
      <BaseButton
        :disabled="overlay.problems.length > 0"
        @click="overlay.copyObsUrl()"
      >
        {{ t('cloudOverlays.copyObsUrl') }}
      </BaseButton>
      <span
        v-if="overlay.copied"
        class="cloud-overlays__note"
      >
        {{ t('cloudOverlays.obsCopied') }}
      </span>
      <span
        v-if="overlay.overlayId"
        class="cloud-overlays__note"
      >
        {{ t('cloudOverlays.savedAs', { id: overlay.overlayId, revision: overlay.revision ?? 0 }) }}
      </span>
    </div>
  </section>
</template>

<style scoped>
.cloud-overlays { display:grid; gap:var(--space-3); min-width:0; }
.cloud-overlays__title { margin:0; }
.cloud-overlays__info { display:inline-flex; margin-left:var(--space-2); vertical-align:middle; color:var(--color-text-muted); font-weight:normal; }
.cloud-overlays__note { margin:0; color:var(--color-text-muted); }
.cloud-overlays__row { display:flex; flex-wrap:wrap; gap:var(--space-2); align-items:center; min-width:0; }
.cloud-overlays__grow { flex:1 1 180px; min-width:0; }
.cloud-overlays__block { display:grid; gap:var(--space-2); padding:var(--space-2); border:1px solid var(--color-border); border-radius:var(--radius-md); min-width:0; }
.cloud-overlays__list, .cloud-overlays__problems { list-style:none; margin:0; padding:0; display:grid; gap:var(--space-2); }
</style>
