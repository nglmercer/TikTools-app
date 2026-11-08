<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { AudioEffectGrant, EffectGrants, HttpEffectGrant } from '@/entities/collaboration/api/types.ts';
import { splitIds } from '@/entities/collaboration/lib/grants.ts';
import { publicEffectRenderFields } from '@/entities/collaboration/lib/effects.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseTextArea from '@/shared/ui/text-area/BaseTextArea.vue';

defineProps<{ disabled?: boolean }>();
const model = defineModel<EffectGrants>();
const { t } = useI18n();
const listFields = ['methods', 'paths', 'queryFields', 'headerNames', 'renderFields', 'bodyFields'] as const;
const numeric = (event: Event) => Number((event.target as HTMLInputElement).value);
const text = (event: Event) => (event.target as HTMLInputElement).value;
const copy = (): EffectGrants => JSON.parse(JSON.stringify(model.value ?? { http: [], audio: [], emitTopics: [] })) as EffectGrants;
function update(value: EffectGrants): void { model.value = value.http.length || value.audio.length || value.emitTopics.length ? value : undefined; }
function patchHttp(index: number, patch: Partial<HttpEffectGrant>): void { const value = copy(); Object.assign(value.http[index]!, patch); update(value); }
function patchAudio(index: number, patch: Partial<AudioEffectGrant>): void { const value = copy(); Object.assign(value.audio[index]!, patch); update(value); }
function addHttp(): void {
  const value = copy();
  value.http.push({ origin: 'https://example.com', methods: ['POST'], paths: ['/'], queryFields: [], headerNames: [], renderFields: [], bodyFields: [], allowTextBody: false, maxBodyBytes: 0, maxResponseBytes: 1024, allowPrivateNetwork: false });
  update(value);
}
function addAudio(): void {
  const value = copy(); value.audio.push({ fileRefs: [], output: 'host-default', maxVolume: 0.5, overlaps: ['drop'] }); update(value);
}
function remove(kind: 'http' | 'audio', index: number): void { const value = copy(); value[kind].splice(index, 1); update(value); }
function topics(raw: string): void { const value = copy(); value.emitTopics = splitIds(raw); update(value); }
</script>

<template>
  <div class="effect-grant-editor">
    <p>{{ t('collaboration.effectRulesHint') }}</p>
    <details><summary>{{ t('collaboration.publicEffectFields') }}</summary><pre>{{ publicEffectRenderFields.join('\n') }}</pre></details>
    <fieldset
      v-for="(rule, index) in model?.http ?? []"
      :key="`http-${index}`"
      :disabled="disabled"
    >
      <legend>{{ t('collaboration.httpRule', { number: index + 1 }) }}</legend>
      <label>{{ t('collaboration.effectOrigin') }}<input
        :value="rule.origin"
        maxlength="2048"
        @input="patchHttp(index, { origin: text($event) })"
      ></label>
      <label
        v-for="field in listFields"
        :key="field"
      >
        {{ t(`collaboration.effectFields.${field}`) }}
        <BaseTextArea
          :model-value="rule[field].join('\n')"
          :aria-label="t(`collaboration.effectFields.${field}`)"
          :rows="2"
          maxlength="65536"
          :disabled="disabled"
          @update:model-value="patchHttp(index, { [field]: splitIds($event) })"
        />
      </label>
      <label>{{ t('collaboration.maxBodyBytes') }}<input
        type="number"
        min="0"
        max="262144"
        :value="rule.maxBodyBytes"
        @input="patchHttp(index, { maxBodyBytes: numeric($event) })"
      ></label>
      <label>{{ t('collaboration.maxResponseBytes') }}<input
        type="number"
        min="1"
        max="2097152"
        :value="rule.maxResponseBytes"
        @input="patchHttp(index, { maxResponseBytes: numeric($event) })"
      ></label>
      <BaseCheckbox
        :model-value="rule.allowTextBody"
        :disabled="disabled"
        @update:model-value="patchHttp(index, { allowTextBody: $event })"
      >
        {{ t('collaboration.allowTextBody') }}
      </BaseCheckbox>
      <BaseCheckbox
        :model-value="rule.allowPrivateNetwork"
        :disabled="disabled"
        @update:model-value="patchHttp(index, { allowPrivateNetwork: $event })"
      >
        {{ t('collaboration.allowPrivateNetwork') }}
      </BaseCheckbox>
      <BaseButton
        :disabled="disabled"
        @click="remove('http', index)"
      >
        {{ t('collaboration.removeEffectRule') }}
      </BaseButton>
    </fieldset>
    <BaseButton
      :disabled="disabled || (model?.http.length ?? 0) >= 32"
      @click="addHttp"
    >
      {{ t('collaboration.addHttpRule') }}
    </BaseButton>
    <fieldset
      v-for="(rule, index) in model?.audio ?? []"
      :key="`audio-${index}`"
      :disabled="disabled"
    >
      <legend>{{ t('collaboration.audioRule', { number: index + 1 }) }}</legend>
      <label>{{ t('collaboration.effectFileRefs') }}<BaseTextArea
        :model-value="rule.fileRefs.join('\n')"
        :aria-label="t('collaboration.effectFileRefs')"
        :rows="3"
        maxlength="262144"
        :disabled="disabled"
        @update:model-value="patchAudio(index, { fileRefs: splitIds($event) })"
      /></label>
      <p>{{ t('collaboration.effectOutput') }}: {{ rule.output }}</p>
      <label>{{ t('collaboration.maxVolume') }}<input
        type="number"
        min="0"
        max="1"
        step="0.05"
        :value="rule.maxVolume"
        @input="patchAudio(index, { maxVolume: numeric($event) })"
      ></label>
      <BaseCheckbox
        v-for="overlap in (['allow', 'restart', 'drop'] as const)"
        :key="overlap"
        :model-value="rule.overlaps.includes(overlap)"
        :disabled="disabled"
        @update:model-value="patchAudio(index, { overlaps: $event ? [...rule.overlaps, overlap] : rule.overlaps.filter((v) => v !== overlap) })"
      >
        {{ t(`collaboration.effectOverlaps.${overlap}`) }}
      </BaseCheckbox>
      <BaseButton
        :disabled="disabled"
        @click="remove('audio', index)"
      >
        {{ t('collaboration.removeEffectRule') }}
      </BaseButton>
    </fieldset>
    <BaseButton
      :disabled="disabled || (model?.audio.length ?? 0) >= 32"
      @click="addAudio"
    >
      {{ t('collaboration.addAudioRule') }}
    </BaseButton>
    <label>{{ t('collaboration.effectEmitTopics') }}<BaseTextArea
      :model-value="model?.emitTopics.join('\n') ?? ''"
      :aria-label="t('collaboration.effectEmitTopics')"
      :rows="3"
      maxlength="16640"
      :disabled="disabled"
      @update:model-value="topics"
    /></label>
  </div>
</template>

<style scoped>
.effect-grant-editor, fieldset { display: grid; gap: var(--space-3); }
fieldset { border: 1px solid var(--color-border); border-radius: var(--radius-md); padding: var(--space-3); }
label { display: grid; gap: var(--space-2); }
input { color: var(--color-text); background: var(--color-bg-field); border: 1px solid var(--color-border); padding: var(--space-2); }
</style>
