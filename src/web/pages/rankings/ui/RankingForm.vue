<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { MetricDescriptor, RankingDefinition, RankingSource } from '@/entities/ranking/types.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseDisclosure from '@/shared/ui/disclosure/BaseDisclosure.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';
import { metricLabel, presetMetrics } from '../lib/metric-labels.ts';
import FormField from './FormField.vue';
import FormSection from './FormSection.vue';
import RankingSourceRow, { type GiftRule } from './RankingSourceRow.vue';

const props = defineProps<{ metrics: MetricDescriptor[]; editing: RankingDefinition | null }>();
const emit = defineEmits<{ saved: [id: string, updated: boolean]; cancel: [] }>();
const { t } = useI18n();
const client = useControlClient();

const initial = props.editing;
const id = ref(initial?.id ?? '');
const creator = ref(initial?.creatorId ?? '');
const room = ref(initial?.roomId ?? '');
const group = ref<RankingDefinition['groupBy']>(initial?.groupBy ?? 'user');
const limit = ref(initial?.limit ?? 10);
const windowType = ref<'session' | 'rolling' | 'daily'>(initial?.window.type ?? 'session');
const duration = ref(initial?.window.type === 'rolling' ? initial.window.durationMs / 1000 : 60);
const maxEvents = ref(initial?.window.type === 'rolling' ? initial.window.maxEvents : 10000);
const utcOffset = ref(initial?.window.type === 'daily' ? initial.window.utcOffsetMinutes : -new Date().getTimezoneOffset());
const sources = ref<RankingSource[]>(initial?.sources.map((source) => ({ ...source })) ?? []);
const giftOverrides = ref<Record<string, GiftRule[]>>(Object.fromEntries((initial?.sources ?? []).filter((source) => source.source === 'tiktok.gift').map((source) => [sourceKey(source), Object.entries(source.giftMultipliers ?? {}).map(([giftId, multiplier]) => ({ giftId, multiplier }))])));
const limitUsers = ref(!!initial?.userLimits);
const maxContributions = ref(initial?.userLimits?.maxContributions ?? 0);
const cooldownMs = ref(initial?.userLimits?.cooldownMs ?? 0);
const maxUsers = ref(initial?.userLimits?.maxUsers ?? 10000);
const advancedOpen = ref(limitUsers.value);
const resetOnUpdate = ref(false);
const selectedMetric = ref('');
watch(() => props.metrics, (metrics) => {
  const preferred = metrics.find((metric) => metric.source === 'tiktok.like' && metric.metric === 'like.count') ?? metrics[0];
  if (!selectedMetric.value && preferred) selectedMetric.value = metricKey(preferred);
}, { immediate: true });
const customSource = ref('');
const customMetric = ref('value');
const busy = ref(false);
const error = ref('');

const groups = ['user', 'gift', 'event_type', 'option', 'team', 'custom_key'] as const;
const windowOptions = computed(() => (['session', 'daily', 'rolling'] as const).map((value) => ({ value, label: t(`rankingsPage.windowNames.${value}`) })));
const presets = computed(() => presetMetrics.filter((key) => props.metrics.some((metric) => `${metric.source}|${metric.metric}` === key)));

function metricKey(metric: MetricDescriptor): string { return JSON.stringify([metric.source, metric.metric]); }
function sourceKey(source: RankingSource): string { return JSON.stringify([source.source, source.metric]); }
function validInteger(value: number, max: number): boolean { return Number.isSafeInteger(value) && value > 0 && value <= max; }
function hasSource(key: string): boolean { return sources.value.some((source) => `${source.source}|${source.metric}` === key); }

function pushSource(source: string, metric: string): void {
  if (sources.value.some((item) => item.source === source && item.metric === metric)) return;
  sources.value.push({ source, metric, weight: 1, enabled: true });
  if (source === 'tiktok.gift') giftOverrides.value[JSON.stringify([source, metric])] = [];
}
function addSource(): void {
  const metric = props.metrics.find((item) => metricKey(item) === selectedMetric.value);
  if (metric) pushSource(metric.source, metric.metric);
}
/** Plugin emits arrive as `plugin.<pluginId>.<eventType>`; agents use `custom.*`. */
function addCustomSource(): void {
  const source = customSource.value.trim();
  if (!/^(plugin|custom)\.[\w.-]+$/.test(source) || source.length > 128) { error.value = t('rankingsPage.form.invalidCustom'); return; }
  pushSource(source, customMetric.value); customSource.value = '';
}
function removeSource(index: number): void {
  const [removed] = sources.value.splice(index, 1);
  if (removed) delete giftOverrides.value[sourceKey(removed)];
}
async function useLive(): Promise<void> {
  try {
    const status = await client.call<{ uniqueId?: string | null; roomId?: string | null }>('live.status', {});
    if (status?.uniqueId && status.roomId) { creator.value = status.uniqueId; room.value = status.roomId; }
  } catch { /* Not connected: the fields stay editable. */ }
}

function validationError(): string {
  if (!id.value.trim() || !creator.value.trim() || !room.value.trim() || !sources.value.length
    || !validInteger(limit.value, 100000) || sources.value.some((source) => !Number.isSafeInteger(source.weight) || source.weight < 0 || source.weight > 1_000_000_000)
    || (limitUsers.value && (!validInteger(maxUsers.value, 100000) || !Number.isSafeInteger(maxContributions.value) || maxContributions.value < 0 || !Number.isSafeInteger(cooldownMs.value) || cooldownMs.value < 0 || (maxContributions.value === 0 && cooldownMs.value === 0)))
    || (windowType.value === 'rolling' && (!validInteger(duration.value * 1000, 86_400_000_000) || !validInteger(maxEvents.value, 100000)))
    || (windowType.value === 'daily' && (!Number.isSafeInteger(utcOffset.value) || Math.abs(utcOffset.value) > 840))) return t('rankingsPage.form.invalid');
  for (const source of sources.value.filter((item) => item.source === 'tiktok.gift')) {
    const rules = giftOverrides.value[sourceKey(source)] ?? [];
    const ids = rules.map((rule) => rule.giftId.trim());
    if (rules.length > 512 || ids.some((value) => !value || value.length > 128) || new Set(ids).size !== ids.length || rules.some((rule) => !Number.isSafeInteger(rule.multiplier) || rule.multiplier < 0 || rule.multiplier > 1_000_000_000)) return t('rankingsPage.form.invalidGift');
  }
  const voteSource = sources.value.some((source) => source.source === 'competition.vote');
  const external = (source: RankingSource) => /^(plugin|custom)\./.test(source.source);
  if ((voteSource && (group.value !== 'option' || windowType.value !== 'session' || limitUsers.value || sources.value.some((source) => !['competition.vote', 'tiktok.like', 'tiktok.share', 'tiktok.gift'].includes(source.source))))
    || (!voteSource && group.value === 'option' && !sources.value.every(external))
    || (['team', 'custom_key'].includes(group.value) && !sources.value.every(external))
    || (group.value === 'gift' && sources.value.some((source) => source.source !== 'tiktok.gift'))) return t('rankingsPage.form.invalidGroup');
  return '';
}

async function submit(): Promise<void> {
  error.value = validationError();
  if (error.value) return;
  const definition: RankingDefinition = { ...initial, schemaVersion: 1, id: id.value.trim(), creatorId: creator.value.trim(), roomId: room.value.trim(), groupBy: group.value,
    window: windowType.value === 'session' ? { type: 'session' } : windowType.value === 'daily' ? { type: 'daily', utcOffsetMinutes: utcOffset.value } : { type: 'rolling', durationMs: duration.value * 1000, maxEvents: maxEvents.value },
    sources: sources.value.map((source) => {
      const result = { ...source };
      const rules = source.source === 'tiktok.gift' ? giftOverrides.value[sourceKey(source)] ?? [] : [];
      if (rules.length) result.giftMultipliers = Object.fromEntries(rules.map((rule) => [rule.giftId.trim(), rule.multiplier]));
      else delete result.giftMultipliers;
      return result;
    }), limit: limit.value, capacity: initial?.capacity ?? 100000 };
  if (limitUsers.value) definition.userLimits = { maxContributions: maxContributions.value || null, cooldownMs: cooldownMs.value || null, maxUsers: maxUsers.value };
  else delete definition.userLimits;
  busy.value = true;
  try {
    if (initial) await client.call('rankings.update', { definition, reset: resetOnUpdate.value });
    else await client.call('rankings.create', { ...definition });
    emit('saved', definition.id, !!initial);
  } catch (failure) { error.value = errorMessage(failure); } finally { busy.value = false; }
}

onMounted(() => { if (!initial) void useLive(); });
</script>

<template>
  <BasePanel :title="editing ? t('rankingsPage.form.editTitle') : t('rankingsPage.form.createTitle')">
    <form
      class="ranking-form"
      @submit.prevent="submit"
    >
      <p
        v-if="error"
        class="ranking-form__error"
        role="alert"
      >
        {{ error }}
      </p>
      <FormSection :title="t('rankingsPage.form.step1')">
        <div class="row">
          <FormField :label="t('rankingsPage.form.id')">
            <input
              v-model="id"
              maxlength="128"
              :disabled="!!editing"
              placeholder="top-gifters"
              required
            >
          </FormField>
          <FormField :label="t('rankingsPage.form.creator')">
            <input
              v-model="creator"
              maxlength="256"
              required
            >
          </FormField>
          <FormField :label="t('rankingsPage.form.room')">
            <input
              v-model="room"
              maxlength="256"
              required
            >
          </FormField>
          <BaseButton @click="useLive">
            {{ t('rankingsPage.form.useLive') }}
          </BaseButton>
        </div>
      </FormSection>

      <FormSection :title="t('rankingsPage.form.step2')">
        <div class="chips">
          <button
            v-for="preset in presets"
            :key="preset"
            type="button"
            class="chip"
            :class="{ 'chip--on': hasSource(preset) }"
            @click="pushSource(...(preset.split('|') as [string, string]))"
          >
            + {{ metricLabel(t, ...(preset.split('|') as [string, string])) }}
          </button>
        </div>
        <ul
          v-if="sources.length"
          class="sources"
        >
          <RankingSourceRow
            v-for="(source, index) in sources"
            :key="sourceKey(source)"
            v-model:source="sources[index]!"
            v-model:overrides="giftOverrides[sourceKey(source)]"
            @remove="removeSource(index)"
          />
        </ul>
        <BaseDisclosure :title="t('rankingsPage.form.otherMetric')">
          <div class="row">
            <FormField :label="t('rankingsPage.form.otherMetric')">
              <select v-model="selectedMetric">
                <option
                  v-for="metric in metrics"
                  :key="metricKey(metric)"
                  :value="metricKey(metric)"
                >
                  {{ metricLabel(t, metric.source, metric.metric) }}
                </option>
              </select>
            </FormField>
            <BaseButton @click="addSource">
              {{ t('rankingsPage.form.addMetric') }}
            </BaseButton>
          </div>
          <div class="row">
            <FormField :label="t('rankingsPage.form.customSource')">
              <input
                v-model="customSource"
                maxlength="128"
                placeholder="plugin.my-plugin.answer"
              >
            </FormField>
            <FormField :label="t('rankingsPage.form.customMetric')">
              <select v-model="customMetric">
                <option value="value">
                  value
                </option>
                <option value="event.count">
                  event.count
                </option>
              </select>
            </FormField>
            <BaseButton @click="addCustomSource">
              {{ t('rankingsPage.form.addCustomSource') }}
            </BaseButton>
          </div>
        </BaseDisclosure>
      </FormSection>

      <FormSection :title="t('rankingsPage.form.step3')">
        <div class="row">
          <FormField :label="t('rankingsPage.form.groupBy')">
            <select v-model="group">
              <option
                v-for="value in groups"
                :key="value"
                :value="value"
              >
                {{ t(`rankingsPage.groups.${value}`) }}
              </option>
            </select>
          </FormField>
          <FormField
            :label="t('rankingsPage.form.window')"
            group
          >
            <BaseSegmented
              v-model="windowType"
              :options="windowOptions"
            />
          </FormField>
        </div>
        <div
          v-if="windowType === 'rolling'"
          class="row"
        >
          <FormField :label="t('rankingsPage.form.duration')">
            <input
              v-model.number="duration"
              type="number"
              min="1"
              max="86400000"
            >
          </FormField>
          <FormField :label="t('rankingsPage.form.maxEvents')">
            <input
              v-model.number="maxEvents"
              type="number"
              min="1"
              max="100000"
            >
          </FormField>
        </div>
        <FormField
          v-if="windowType === 'daily'"
          :label="t('rankingsPage.form.utcOffset')"
        >
          <input
            v-model.number="utcOffset"
            type="number"
            min="-840"
            max="840"
            step="15"
          >
        </FormField>
      </FormSection>

      <BaseDisclosure
        v-model:open="advancedOpen"
        :title="t('rankingsPage.form.advanced')"
      >
        <div class="advanced">
          <FormField :label="t('rankingsPage.form.topEntries')">
            <input
              v-model.number="limit"
              type="number"
              min="1"
              max="100000"
            >
          </FormField>
          <BaseCheckbox v-model="limitUsers">
            {{ t('rankingsPage.form.limitUsers') }}
          </BaseCheckbox>
          <div
            v-if="limitUsers"
            class="row"
          >
            <FormField :label="t('rankingsPage.form.maxContributions')">
              <input
                v-model.number="maxContributions"
                type="number"
                min="0"
                step="1"
              >
            </FormField>
            <FormField :label="t('rankingsPage.form.cooldown')">
              <input
                v-model.number="cooldownMs"
                type="number"
                min="0"
                step="1"
              >
            </FormField>
            <FormField :label="t('rankingsPage.form.maxUsers')">
              <input
                v-model.number="maxUsers"
                type="number"
                min="1"
                max="100000"
                step="1"
              >
            </FormField>
          </div>
        </div>
      </BaseDisclosure>

      <BaseCheckbox
        v-if="editing"
        v-model="resetOnUpdate"
      >
        {{ t('rankingsPage.form.resetOnUpdate') }}
      </BaseCheckbox>
      <div class="actions">
        <BaseButton
          type="submit"
          variant="primary"
          :disabled="busy || !sources.length"
        >
          {{ editing ? t('rankingsPage.form.save') : t('rankingsPage.form.create') }}
        </BaseButton>
        <BaseButton @click="emit('cancel')">
          {{ t('rankingsPage.form.cancel') }}
        </BaseButton>
      </div>
    </form>
  </BasePanel>
</template>

<style scoped>
.ranking-form { display: grid; gap: var(--space-4); }
.ranking-form__error { margin: 0; color: var(--color-danger, #fb7185); }
.row { display: flex; flex-wrap: wrap; align-items: end; gap: var(--space-3); }
.chips { display: flex; flex-wrap: wrap; gap: var(--space-2); }
.chip { padding: 4px 12px; border-radius: 999px; color: inherit; background: var(--color-bg-control); border: 1px solid var(--color-border); cursor: pointer; }
.chip:hover, .chip--on { border-color: var(--color-accent, #3b82f6); }
.chip--on { background: color-mix(in srgb, var(--color-accent, #3b82f6) 18%, transparent); }
.sources { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--space-2); }
.advanced { display: grid; gap: var(--space-3); padding: var(--space-2) 0; }
.actions { display: flex; gap: var(--space-2); }
</style>
