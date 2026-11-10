<script setup lang="ts">
import { computed, ref, unref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { MessageSchema } from '@/shared/i18n/i18n.ts';
import type { ProposalConfirmation, ProposalModel } from '../model/proposals.ts';
import type { AutomationModel, AutomationRow } from '../model/automations.ts';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';

const props = defineProps<{ model: ProposalModel; automation?: AutomationModel }>();
const { t, tm } = useI18n();
const labels = computed(() => tm('controller.proposalBuilder.labels') as MessageSchema['controller']['proposalBuilder']['labels']);

const name = ref('');
const actionType = ref<'core.log' | 'core.delay' | 'core.points' | 'core.emit' | 'core.fetch' | 'audio.play' | 'core.code'>('core.log');
const message = ref('');
const milliseconds = ref('1000');
const viewer = ref('');
const viewerTarget = ref<'specific' | 'event'>('specific');
const delta = ref('');
const trigger = ref<'tiktok.chat' | 'tiktok.gift' | 'tiktok.like' | 'tiktok.follow' | 'tiktok.share' | 'tiktok.join' | 'custom'>('tiktok.chat');
const emitTopic = ref(''); const emitData = ref('{}'); const customTopic = ref('');
const fetchUrl = ref(''); const fetchMethod = ref<'GET' | 'POST' | 'PUT' | 'DELETE'>('POST');
const fetchHeaders = ref('{}'); const fetchBody = ref(''); const fetchTimeout = ref('5000'); const fetchResponse = ref('2097152'); const fetchPrivate = ref(false);
const audioPath = ref(''); const audioVolume = ref('1'); const audioOverlap = ref<'allow' | 'restart' | 'drop'>('allow');
const scriptSource = ref('');
const contains = ref('');
const giftContains = ref('');
const actionId = ref('');
const selectedActionIds = ref<string[]>([]);
const runMode = ref<'all' | 'random'>('all');
const actionSearch = ref('');
const cooldown = ref('0');
const cooldownScope = ref<'global' | 'user'>('global');
const loadedSource = ref<{ text: string; record: Record<string, unknown> } | null>(null);
const loadError = ref(false);
const encoder = new TextEncoder();
const triggers = new Set(['tiktok.chat', 'tiktok.gift', 'tiktok.like', 'tiktok.follow', 'tiktok.share', 'tiktok.join']);
const validTopic = (value: string) => value.length > 0 && value.length <= 64 && !/[^a-z0-9._-]/.test(value);
const object = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value);
function boundedEmitData(value: unknown): value is Record<string, unknown> {
  if (!object(value)) return false;
  const pending: { value: unknown; depth: number }[] = [{ value, depth: 0 }];
  let count = 0;
  while (pending.length) {
    const current = pending.pop()!;
    if (++count > 8192 || current.depth > 32 || (typeof current.value === 'number' && !Number.isFinite(current.value))) return false;
    if (current.value && typeof current.value === 'object') {
      for (const child of Object.values(current.value)) pending.push({ value: child, depth: current.depth + 1 });
    }
  }
  try { return encoder.encode(JSON.stringify(value)).length <= 64 * 1024; } catch { return false; }
}
function httpHeaders(text: string): Record<string, string> | null {
  if (encoder.encode(text).length > 64 * 1024) return null;
  let parsed: unknown; try { parsed = JSON.parse(text); } catch { return null; }
  if (!object(parsed) || Object.keys(parsed).length > 64 || !Object.entries(parsed).every(([key, value]) => !!key && !/[^!#$%&'*+.^_`|~0-9a-z-]/.test(key) && typeof value === 'string' && encoder.encode(value).length <= 8192)) return null;
  return parsed as Record<string, string>;
}
function validHttpUrl(text: string, allowPrivate: boolean): boolean {
  if (!text || text.trim() !== text || encoder.encode(text).length > 8192) return false;
  try {
    const url = new URL(text);
    return (url.protocol === 'https:' || (url.protocol === 'http:' && allowPrivate)) && !!url.hostname && !url.username && !url.password && !url.hash;
  } catch { return false; }
}
const integer = (value: string, max: number): number | null => {
  if (!/^(0|[1-9]\d*)$/.test(value)) return null;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed <= max ? parsed : null;
};
const canonicalId = (value: string) => value.length > 0 && value.trim() === value
  && encoder.encode(value).length <= 128 && !/[\p{Cc}*]/u.test(value);
const validAudioPath = (value: string) => !!value && value.trim() === value && encoder.encode(value).length <= 4096 && !/[\p{Cc}]/u.test(value) && !value.includes('{{') && !value.includes('}}');
const parseVolume = (value: string): number | null => {
  if (value.trim() !== value || !/^(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(value)) return null;
  const n = Number(value); return Number.isFinite(n) && n >= 0 && n <= 1 ? n : null;
};
const actionIds = computed(() => [...selectedActionIds.value, ...(actionId.value ? [actionId.value] : [])]);
const actionsValid = computed(() => actionIds.value.length <= 512 && actionIds.value.every(canonicalId) && new Set(actionIds.value).size === actionIds.value.length);
function addAction(): void {
  if (props.model.busy.value || !props.model.editable.value || !actionId.value || !actionsValid.value) return;
  selectedActionIds.value = [...selectedActionIds.value, actionId.value];
  actionId.value = '';
}
function removeAction(index: number): void {
  if (props.model.busy.value || !props.model.editable.value || index < 0 || index >= selectedActionIds.value.length) return;
  selectedActionIds.value = selectedActionIds.value.filter((_, i) => i !== index);
}
function moveAction(index: number, delta: -1 | 1): void {
  const next = index + delta;
  if (props.model.busy.value || !props.model.editable.value || index < 0 || next < 0 || index >= selectedActionIds.value.length || next >= selectedActionIds.value.length) return;
  const reordered = [...selectedActionIds.value];
  [reordered[index], reordered[next]] = [reordered[next]!, reordered[index]!];
  selectedActionIds.value = reordered;
}
const literalViewer = (value: string) => canonicalId(value) && !value.startsWith('@') && !value.includes('{{') && !value.includes('}}');
const pointsChange = (value: string): number | null => {
  if (!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(value)) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed !== 0 && Math.abs(parsed) <= 1_000_000_000 ? parsed : null;
};
const actionsReady = computed(() => props.automation ? unref(props.automation.ready) : false);
const matchingActions = computed(() => {
  if (!props.automation || !actionsReady.value) return [];
  const search = actionSearch.value.trim().toLocaleLowerCase();
  return unref(props.automation.rows).filter((row) => row.summary.kind === 'action' &&
    (!search || row.summary.id.toLocaleLowerCase().includes(search) || row.summary.name.toLocaleLowerCase().includes(search)));
});
function useAction(row: AutomationRow): void {
  if (!actionsReady.value || props.model.busy.value || !props.model.editable.value) return;
  if (!matchingActions.value.some((current) => current.version.id === row.version.id && current.version.revision === row.version.revision)) return;
  actionId.value = row.summary.id;
}
function loadFields(): void {
  if (!props.model.editable.value || props.model.busy.value) return;
  loadError.value = true;
  let record: unknown;
  try { record = JSON.parse(props.model.text.value); } catch { return; }
  if (!object(record) || record['id'] !== props.model.id.value || record['enabled'] !== false || typeof record['name'] !== 'string') return;
  if (props.model.kind.value === 'action') {
    if (record['schemaVersion'] !== undefined && record['schemaVersion'] !== 2) return;
    const config = record['config'];
    if (!object(config)) return;
    const type = record['typeId'];
    if (type === 'core.log' && typeof config['message'] === 'string') {
      actionType.value = type; message.value = config['message'];
    } else if (type === 'core.delay' && typeof config['ms'] === 'number' && integer(String(config['ms']), 60_000) !== null) {
      actionType.value = type; milliseconds.value = String(config['ms']);
    } else if (type === 'core.points' && typeof config['uniqueId'] === 'string' && typeof config['delta'] === 'number' && pointsChange(String(config['delta'])) !== null && (config['uniqueId'] === '{{event.user.uniqueId}}' || literalViewer(config['uniqueId']))) {
      actionType.value = type; viewerTarget.value = config['uniqueId'] === '{{event.user.uniqueId}}' ? 'event' : 'specific';
      viewer.value = viewerTarget.value === 'specific' ? config['uniqueId'] : ''; delta.value = String(config['delta']);
    } else if (type === 'core.emit' && typeof config['type'] === 'string' && validTopic(config['type']) && boundedEmitData(config['data'] ?? {})) {
      actionType.value = type; emitTopic.value = config['type']; emitData.value = JSON.stringify(config['data'] ?? {}, null, 2);
    } else if (type === 'core.fetch') {
      const url = config['url']; const method = config['method'] ?? 'POST'; const headers = config['headers'] ?? {}; const body = config['body'] ?? '';
      const timeout = config['timeoutMs'] ?? 5000; const response = config['maxResponseBytes'] ?? 2_097_152; const allowPrivate = config['allowPrivateNetwork'] ?? false;
      if (typeof url !== 'string' || typeof allowPrivate !== 'boolean' || !validHttpUrl(url, allowPrivate) || typeof method !== 'string' || !['GET','POST','PUT','DELETE'].includes(method) || !object(headers) || !Object.values(headers).every((value) => typeof value === 'string') || !httpHeaders(JSON.stringify(headers)) || typeof body !== 'string' || encoder.encode(body).length > 256 * 1024 || typeof timeout !== 'number' || integer(String(timeout), 120_000) === null || timeout < 100 || typeof response !== 'number' || integer(String(response), 2_097_152) === null || response < 1 || (config['emitResponseAs'] !== undefined && config['emitResponseAs'] !== '')) return;
      actionType.value = type; fetchUrl.value = url; fetchMethod.value = method as typeof fetchMethod.value; fetchHeaders.value = JSON.stringify(headers, null, 2);
      fetchBody.value = body; fetchTimeout.value = String(timeout); fetchResponse.value = String(response); fetchPrivate.value = allowPrivate;
    } else if (type === 'audio.play' || type === 'core.audio.play') {
      const key = ['fileRef', 'file', 'filePath', 'path'].find((k) => Object.hasOwn(config, k));
      const raw = key ? config[key] : undefined;
      const path = object(raw) ? raw['path'] : raw; const volume = config['volume'] ?? 1; const overlap = config['overlap'] ?? 'allow';
      if (typeof path !== 'string' || !validAudioPath(path) || typeof volume !== 'number' || parseVolume(String(volume)) === null || typeof overlap !== 'string' || !['allow','restart','drop'].includes(overlap)) return;
      actionType.value = 'audio.play'; audioPath.value = path; audioVolume.value = String(volume); audioOverlap.value = overlap as typeof audioOverlap.value;
    } else if (type === 'core.code' && typeof config['source'] === 'string' && !!config['source'] && encoder.encode(config['source']).length <= 128 * 1024) {
      actionType.value = type; scriptSource.value = config['source'];
    } else return;
  } else {
    if (record['schemaVersion'] !== undefined && record['schemaVersion'] !== 1) return;
    const type = record['trigger']; const ids = record['actionIds']; const filters = record['filters'];
    const wait = record['cooldownMs'] ?? 0; const scope = record['cooldownScope'] ?? 'global'; const mode = record['runMode'] ?? 'all';
    if (typeof type !== 'string' || !(triggers.has(type) || (validTopic(type) && !type.startsWith('tiktok.'))) || !Array.isArray(ids) || ids.length > 512 || !ids.every((id) => typeof id === 'string' && canonicalId(id)) || new Set(ids).size !== ids.length || !Array.isArray(filters) || filters.length > 1 || typeof wait !== 'number' || integer(String(wait), 86_400_000) === null || !['global', 'user'].includes(String(scope)) || !['all', 'random'].includes(String(mode))) return;
    const filter = filters[0];
    if (filter !== undefined && (!object(filter) || Object.keys(filter).some((key) => !['path', 'operator', 'value'].includes(key)) || filter['operator'] !== 'contains' || typeof filter['value'] !== 'string' || !filter['value'] || !((type === 'tiktok.chat' && filter['path'] === 'event.data.comment') || (type === 'tiktok.gift' && filter['path'] === 'event.data.giftName')))) return;
    trigger.value = triggers.has(type) ? type as typeof trigger.value : 'custom'; customTopic.value = trigger.value === 'custom' ? type : ''; selectedActionIds.value = [...ids]; actionId.value = '';
    contains.value = type === 'tiktok.chat' && object(filter) ? filter['value'] as string : '';
    giftContains.value = type === 'tiktok.gift' && object(filter) ? filter['value'] as string : '';
    cooldown.value = String(wait); cooldownScope.value = scope as typeof cooldownScope.value; runMode.value = mode as typeof runMode.value;
  }
  name.value = record['name'];
  loadedSource.value = { text: props.model.text.value, record }; loadError.value = false;
}
const draft = computed<string | null>(() => {
  const id = props.model.id.value;
  if (loadedSource.value && loadedSource.value.text !== props.model.text.value) return null;
  const source = loadedSource.value?.record ?? {};
  if (!props.model.editable.value || !canonicalId(id) || !name.value.trim() || encoder.encode(name.value).length > 256) return null;
  if (props.model.kind.value === 'action') {
    if (actionType.value === 'core.log') {
      if (!message.value.trim() || encoder.encode(message.value).length > 8192) return null;
      return JSON.stringify({ ...source, id, name: name.value, enabled: false, typeId: 'core.log', config: { ...(source['typeId'] === 'core.log' && object(source['config']) ? source['config'] : {}), message: message.value }, schemaVersion: 2 }, null, 2);
    }
    if (actionType.value === 'core.delay') {
      const ms = integer(milliseconds.value, 60_000);
      if (ms === null) return null;
      return JSON.stringify({ ...source, id, name: name.value, enabled: false, typeId: 'core.delay', config: { ...(source['typeId'] === 'core.delay' && object(source['config']) ? source['config'] : {}), ms }, schemaVersion: 2 }, null, 2);
    }
    if (actionType.value === 'core.points') {
      const amount = pointsChange(delta.value);
      if (amount === null || !['specific', 'event'].includes(viewerTarget.value) || (viewerTarget.value === 'specific' && !literalViewer(viewer.value))) return null;
      const uniqueId = viewerTarget.value === 'event' ? '{{event.user.uniqueId}}' : viewer.value;
      return JSON.stringify({ ...source, id, name: name.value, enabled: false, typeId: 'core.points', config: { ...(source['typeId'] === 'core.points' && object(source['config']) ? source['config'] : {}), uniqueId, delta: amount }, schemaVersion: 2 }, null, 2);
    }
    if (actionType.value === 'core.emit') {
      if (!validTopic(emitTopic.value) || encoder.encode(emitData.value).length > 64 * 1024) return null;
      let data: unknown; try { data = JSON.parse(emitData.value); } catch { return null; }
      if (!boundedEmitData(data)) return null;
      return JSON.stringify({ ...source, id, name: name.value, enabled: false, typeId: 'core.emit', config: { ...(source['typeId'] === 'core.emit' && object(source['config']) ? source['config'] : {}), type: emitTopic.value, data }, schemaVersion: 2 }, null, 2);
    }
    if (actionType.value === 'core.fetch') {
      const headers = httpHeaders(fetchHeaders.value); const timeoutMs = integer(fetchTimeout.value, 120_000); const maxResponseBytes = integer(fetchResponse.value, 2_097_152);
      if (!validHttpUrl(fetchUrl.value, fetchPrivate.value) || !['GET','POST','PUT','DELETE'].includes(fetchMethod.value) || !headers || timeoutMs === null || timeoutMs < 100 || maxResponseBytes === null || maxResponseBytes < 1 || encoder.encode(fetchBody.value).length > 256 * 1024) return null;
      return JSON.stringify({ ...source, id, name: name.value, enabled: false, typeId: 'core.fetch', config: { ...(source['typeId'] === 'core.fetch' && object(source['config']) ? source['config'] : {}), url: fetchUrl.value, method: fetchMethod.value, headers, body: fetchBody.value || null, timeoutMs, maxResponseBytes, allowPrivateNetwork: fetchPrivate.value }, schemaVersion: 2 }, null, 2);
    }
    if (actionType.value === 'audio.play') {
      const volume = parseVolume(String(audioVolume.value));
      if (!validAudioPath(audioPath.value) || volume === null || !['allow','restart','drop'].includes(audioOverlap.value)) return null;
      const sameType = source['typeId'] === 'audio.play' || source['typeId'] === 'core.audio.play';
      const original = sameType && object(source['config']) ? source['config'] : {};
      const key = ['fileRef', 'file', 'filePath', 'path'].find((k) => Object.hasOwn(original, k)) ?? 'fileRef';
      const old = original[key]; const oldPath = object(old) ? old['path'] : old;
      const file = object(old) ? (oldPath === audioPath.value ? old : {path: audioPath.value}) : audioPath.value;
      return JSON.stringify({ ...source, id, name: name.value, enabled: false, typeId: sameType ? source['typeId'] : 'audio.play', config: { ...original, [key]: file, volume, overlap: audioOverlap.value }, schemaVersion: 2 }, null, 2);
    }
    if (actionType.value === 'core.code') {
      if (!scriptSource.value || encoder.encode(scriptSource.value).length > 128 * 1024) return null;
      return JSON.stringify({ ...source, id, name: name.value, enabled: false, typeId: 'core.code', config: { ...(source['typeId'] === 'core.code' && object(source['config']) ? source['config'] : {}), source: scriptSource.value }, schemaVersion: 2 }, null, 2);
    }
    return null;
  }
  const wait = integer(cooldown.value, 86_400_000);
  const eventType = trigger.value === 'custom' ? customTopic.value : trigger.value;
  if (wait === null || !(trigger.value === 'custom' ? validTopic(eventType) && !eventType.startsWith('tiktok.') : triggers.has(eventType)) || !actionsValid.value || !['all', 'random'].includes(runMode.value) ||
    !['global', 'user'].includes(cooldownScope.value) ||
    (trigger.value === 'tiktok.chat' && encoder.encode(contains.value).length > 500) ||
    (trigger.value === 'tiktok.gift' && encoder.encode(giftContains.value).length > 500)) return null;
  const filters = trigger.value === 'tiktok.chat' && contains.value
    ? [{ path: 'event.data.comment', operator: 'contains', value: contains.value }]
    : trigger.value === 'tiktok.gift' && giftContains.value
      ? [{ path: 'event.data.giftName', operator: 'contains', value: giftContains.value }]
      : [];
  return JSON.stringify({ ...source, id, name: name.value, enabled: false, trigger: eventType,
    actionIds: actionIds.value, filters, cooldownMs: wait, cooldownScope: cooldownScope.value, runMode: runMode.value, schemaVersion: 1 }, null, 2);
});

type Capture = { before: ProposalConfirmation; generated: string };
const capture = ref<Capture | null>(null);
const open = computed({ get: () => capture.value !== null, set: (value: boolean) => { if (!value) capture.value = null; } });
const current = computed(() => capture.value !== null && draft.value === capture.value.generated && props.model.current(capture.value.before));
function prepare(): void {
  const before = props.model.prepare('save');
  if (before && draft.value !== null) capture.value = { before, generated: draft.value };
}
function apply(): void {
  if (!capture.value || !current.value) return;
  const generated = capture.value.generated;
  capture.value = null;
  props.model.edit(generated);
  if (loadedSource.value) loadedSource.value = { text: generated, record: JSON.parse(generated) as Record<string, unknown> };
}
// Source and form state are both private to the current connection/selection.
watch(() => [props.model.identityKey.value, props.model.kind.value, props.model.id.value], () => {
  capture.value = null; loadedSource.value = null; loadError.value = false; name.value = ''; message.value = ''; actionId.value = ''; selectedActionIds.value = []; actionSearch.value = ''; contains.value = ''; giftContains.value = ''; viewer.value = ''; delta.value = '';
  actionType.value = 'core.log'; viewerTarget.value = 'specific'; runMode.value = 'all'; milliseconds.value = '1000'; trigger.value = 'tiktok.chat'; cooldown.value = '0'; cooldownScope.value = 'global'; emitTopic.value = ''; emitData.value = '{}'; customTopic.value = ''; fetchUrl.value = ''; fetchMethod.value = 'POST'; fetchHeaders.value = '{}'; fetchBody.value = ''; fetchTimeout.value = '5000'; fetchResponse.value = '2097152'; fetchPrivate.value = false; audioPath.value = ''; audioVolume.value = '1'; audioOverlap.value = 'allow'; scriptSource.value = '';
}, { flush: 'sync' });
</script>

<template>
  <details class="proposal-builder">
    <summary>{{ labels.title }}</summary>
    <p>{{ labels.note }}</p>
    <p>{{ labels.loadFieldsHint }}</p>
    <button
      type="button"
      :disabled="model.busy.value || !model.editable.value"
      @click="loadFields"
    >
      {{ labels.loadFields }}
    </button>
    <p
      v-if="loadError"
      role="alert"
    >
      {{ labels.loadFieldsError }}
    </p>
    <p
      v-if="loadedSource && loadedSource.text !== model.text.value"
      role="status"
    >
      {{ labels.staleFields }}
    </p>
    <div class="fields">
      <label>{{ labels.name }}<input
        v-model="name"
        maxlength="256"
        :disabled="model.busy.value"
      ></label>
      <template v-if="model.kind.value === 'action'">
        <label>{{ labels.actionType }}<select
          v-model="actionType"
          :disabled="model.busy.value"
        ><option value="core.log">{{ labels.log }}</option><option value="core.delay">{{ labels.delay }}</option><option value="core.points">{{ labels.points }}</option><option value="core.emit">{{ labels.emit }}</option><option value="core.fetch">{{ labels.fetch }}</option><option value="audio.play">{{ labels.audio }}</option><option value="core.code">{{ labels.script }}</option></select></label>
        <label v-if="actionType === 'core.log'">{{ labels.message }}<input
          v-model="message"
          maxlength="8192"
          :disabled="model.busy.value"
        ></label>
        <label v-else-if="actionType === 'core.delay'">{{ labels.milliseconds }}<input
          v-model="milliseconds"
          type="number"
          min="0"
          max="60000"
          step="1"
          :disabled="model.busy.value"
        ></label>
        <template v-else-if="actionType === 'core.emit'">
          <label>{{ labels.topic }}<input
            v-model="emitTopic"
            maxlength="64"
            spellcheck="false"
            :disabled="model.busy.value"
          ></label>
          <label>{{ labels.emitData }}<textarea
            v-model="emitData"
            rows="4"
            spellcheck="false"
            :disabled="model.busy.value"
          /></label>
        </template>
        <template v-else-if="actionType === 'core.fetch'">
          <label>{{ labels.fetchUrl }}<input
            v-model="fetchUrl"
            maxlength="8192"
            spellcheck="false"
            autocomplete="off"
            :disabled="model.busy.value"
          ></label>
          <label>{{ labels.fetchMethod }}<select
            v-model="fetchMethod"
            :disabled="model.busy.value"
          ><option>GET</option><option>POST</option><option>PUT</option><option>DELETE</option></select></label>
          <label>{{ labels.fetchHeaders }}<textarea
            v-model="fetchHeaders"
            rows="4"
            spellcheck="false"
            autocomplete="off"
            :disabled="model.busy.value"
          /></label>
          <label>{{ labels.fetchBody }}<textarea
            v-model="fetchBody"
            rows="4"
            spellcheck="false"
            autocomplete="off"
            :disabled="model.busy.value"
          /></label>
          <label>{{ labels.fetchTimeout }}<input
            v-model="fetchTimeout"
            type="number"
            min="100"
            max="120000"
            step="1"
            :disabled="model.busy.value"
          ></label>
          <label>{{ labels.fetchResponse }}<input
            v-model="fetchResponse"
            type="number"
            min="1"
            max="2097152"
            step="1"
            :disabled="model.busy.value"
          ></label>
          <label><input
            v-model="fetchPrivate"
            type="checkbox"
            :disabled="model.busy.value"
          >{{ labels.fetchPrivate }}</label>
        </template>
        <template v-else-if="actionType === 'audio.play'">
          <label>{{ labels.audioPath }}<input
            v-model="audioPath"
            maxlength="4096"
            spellcheck="false"
            autocomplete="off"
            :disabled="model.busy.value"
          ></label>
          <label>{{ labels.audioVolume }}<input
            v-model="audioVolume"
            type="number"
            min="0"
            max="1"
            step="any"
            :disabled="model.busy.value"
          ></label>
          <label>{{ labels.audioOverlap }}<select
            v-model="audioOverlap"
            :disabled="model.busy.value"
          ><option value="allow">{{ labels.audioAllow }}</option><option value="restart">{{ labels.audioRestart }}</option><option value="drop">{{ labels.audioDrop }}</option></select></label>
        </template>
        <template v-else-if="actionType === 'core.code'">
          <label>{{ labels.scriptSource }}<textarea
            v-model="scriptSource"
            rows="10"
            spellcheck="false"
            autocomplete="off"
            :disabled="model.busy.value"
          /></label>
        </template>
        <template v-else>
          <label>{{ labels.viewerTarget }}<select
            v-model="viewerTarget"
            :disabled="model.busy.value"
          ><option value="specific">{{ labels.specificViewer }}</option><option value="event">{{ labels.triggeringViewer }}</option></select></label>
          <label v-if="viewerTarget === 'specific'">{{ labels.viewer }}<input
            v-model="viewer"
            maxlength="128"
            spellcheck="false"
            autocomplete="off"
            :disabled="model.busy.value"
          ></label>
          <label>{{ labels.delta }}<input
            v-model="delta"
            type="number"
            min="-1000000000"
            max="1000000000"
            step="any"
            :disabled="model.busy.value"
          ></label>
        </template>
      </template>
      <template v-else>
        <label>{{ labels.trigger }}<select
          v-model="trigger"
          :disabled="model.busy.value"
        ><option value="tiktok.chat">{{ labels.chat }}</option><option value="tiktok.gift">{{ labels.gift }}</option><option value="tiktok.like">{{ labels.like }}</option><option value="tiktok.follow">{{ labels.follow }}</option><option value="tiktok.share">{{ labels.share }}</option><option value="tiktok.join">{{ labels.join }}</option><option value="custom">{{ labels.custom }}</option></select></label>
        <label v-if="trigger === 'custom'">{{ labels.topic }}<input
          v-model="customTopic"
          maxlength="64"
          spellcheck="false"
          :disabled="model.busy.value"
        ></label>
        <label v-if="trigger === 'tiktok.chat'">{{ labels.contains }}<input
          v-model="contains"
          maxlength="500"
          :disabled="model.busy.value"
        ></label>
        <label v-if="trigger === 'tiktok.gift'">{{ labels.giftContains }}<input
          v-model="giftContains"
          maxlength="500"
          :disabled="model.busy.value"
        ></label>
        <label>{{ labels.actionId }}<input
          v-model="actionId"
          maxlength="128"
          spellcheck="false"
          :disabled="model.busy.value"
        ></label>
        <label>{{ labels.runMode }}<select
          v-model="runMode"
          :disabled="model.busy.value"
        ><option value="all">{{ labels.runAll }}</option><option value="random">{{ labels.runRandom }}</option></select></label>
        <label>{{ labels.cooldown }}<input
          v-model="cooldown"
          type="number"
          min="0"
          max="86400000"
          step="1"
          :disabled="model.busy.value"
        ></label>
        <label>{{ labels.cooldownScope }}<select
          v-model="cooldownScope"
          :disabled="model.busy.value"
        ><option value="global">{{ labels.cooldownGlobal }}</option><option value="user">{{ labels.cooldownViewer }}</option></select></label>
      </template>
    </div>
    <div
      v-if="model.kind.value === 'event'"
      class="selected-actions"
    >
      <button
        type="button"
        :disabled="model.busy.value || !model.editable.value || !actionId || !actionsValid"
        @click="addAction"
      >
        {{ labels.addAction }}
      </button>
      <p
        v-if="!actionsValid"
        role="status"
      >
        {{ labels.actionError }}
      </p>
      <template v-if="selectedActionIds.length">
        <h4>{{ labels.selectedActions }}</h4>
        <ol>
          <li
            v-for="(id, index) in selectedActionIds"
            :key="id"
          >
            <span>{{ id }}</span>
            <button
              type="button"
              :aria-label="t('controller.proposalBuilder.labels.moveEarlier', { id })"
              :disabled="model.busy.value || !model.editable.value || index === 0"
              @click="moveAction(index, -1)"
            >
              ↑
            </button>
            <button
              type="button"
              :aria-label="t('controller.proposalBuilder.labels.moveLater', { id })"
              :disabled="model.busy.value || !model.editable.value || index === selectedActionIds.length - 1"
              @click="moveAction(index, 1)"
            >
              ↓
            </button>
            <button
              type="button"
              :aria-label="t('controller.proposalBuilder.labels.removeAction', { id })"
              :disabled="model.busy.value || !model.editable.value"
              @click="removeAction(index)"
            >
              ×
            </button>
          </li>
        </ol>
      </template>
    </div>
    <section
      v-if="model.kind.value === 'event' && actionsReady"
      :aria-label="labels.availableActions"
      class="available-actions"
    >
      <h4>{{ labels.availableActions }}</h4>
      <p>{{ labels.actionStatusNote }}</p>
      <label>{{ labels.actionSearch }}<input
        v-model="actionSearch"
        type="search"
        maxlength="128"
        autocomplete="off"
        :disabled="model.busy.value"
      ></label>
      <p v-if="!matchingActions.length">
        {{ labels.noActions }}
      </p>
      <ul v-else>
        <li
          v-for="row in matchingActions.slice(0, 50)"
          :key="row.summary.id"
        >
          <span>{{ row.summary.name }} · {{ row.summary.id }} · {{ row.summary.enabled ? labels.actionEnabled : labels.actionDisabled }}</span>
          <button
            type="button"
            :disabled="model.busy.value || !model.editable.value"
            @click="useAction(row)"
          >
            {{ labels.selectAction }}
          </button>
        </li>
      </ul>
      <p v-if="matchingActions.length > 50">
        {{ labels.actionLimit }}
      </p>
    </section>
    <p v-if="model.kind.value === 'event'">
      {{ labels.actionNote }}
    </p>
    <p v-if="model.kind.value === 'event' && trigger === 'tiktok.gift'">
      {{ labels.giftHint }}
    </p>
    <p v-if="model.kind.value === 'event' && trigger === 'custom'">
      {{ labels.customHint }}
    </p>
    <p v-if="model.kind.value === 'action' && actionType === 'core.emit'">
      {{ labels.emitHint }}
    </p>
    <p v-if="model.kind.value === 'action' && actionType === 'core.fetch'">
      {{ labels.fetchHint }}
    </p>
    <p v-if="model.kind.value === 'action' && actionType === 'audio.play'">
      {{ labels.audioHint }}
    </p>
    <p v-if="model.kind.value === 'action' && actionType === 'core.code'">
      {{ labels.scriptHint }}
    </p>
    <p v-if="model.kind.value === 'event' && cooldown !== '0'">
      {{ labels.cooldownNote }}
    </p>
    <p v-if="model.kind.value === 'action' && actionType === 'core.points'">
      {{ viewerTarget === 'event' ? labels.dynamicPointsNote : labels.pointsNote }}
    </p>
    <p
      v-if="draft === null"
      role="status"
    >
      {{ labels.invalid }}
    </p>
    <button
      type="button"
      :disabled="draft === null || !model.prepare('save')"
      @click="prepare"
    >
      {{ labels.generate }}
    </button>
  </details>
  <BaseConfirmDialog
    v-model:open="open"
    :title="labels.confirm"
    :message="labels.warning"
    :confirm-label="labels.apply"
    :busy="!current"
    @confirm="apply"
  >
    <template v-if="capture">
      <p>{{ labels.preview }}</p><pre>{{ capture.generated }}</pre>
      <p
        v-if="!current"
        role="status"
      >
        {{ labels.changed }}
      </p>
    </template>
  </BaseConfirmDialog>
</template>

<style scoped>
.proposal-builder { margin: 12px 0; padding: 12px; border: 1px solid var(--color-border-subtle); border-radius: 6px; }
.fields { display: flex; flex-wrap: wrap; gap: 12px; }
label { display: flex; flex-direction: column; gap: 6px; flex: 1 1 180px; min-width: 0; }
input, select, textarea, button { padding: 8px; color: inherit; background: var(--color-bg-panel); border: 1px solid var(--color-border-subtle); border-radius: 6px; }
button:disabled { opacity: .5; } pre { max-height: 260px; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; }
.available-actions { margin-top:12px; } .available-actions ul { max-height:14rem; overflow:auto; padding-left:1.5rem; }
.available-actions li { margin:6px 0; overflow-wrap:anywhere; } .available-actions li button { margin-left:8px; }
</style>
