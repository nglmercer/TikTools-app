import { computed, ref, unref, watch } from 'vue';
import type { ProposalModel } from './proposals.ts';
import type { AutomationModel, AutomationRow } from './automations.ts';
import { encoder, triggers, validTopic, object, boundedEmitData, httpHeaders, validHttpUrl, integer, canonicalId, validAudioPath, parseVolume, literalViewer, pointsChange } from './proposal-field-validation.ts';

// Instantiate once in the builder: field panels may unmount, but draft input must survive.
// This hook only generates disabled JSON; applying it requires the root review capture.
export function useProposalBuilderDraft(props: Readonly<{ model: ProposalModel; automation?: AutomationModel }>) {
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

  // Source and form state are both private to the current connection/selection.
  watch(() => [props.model.identityKey.value, props.model.kind.value, props.model.id.value], () => {
    loadedSource.value = null; loadError.value = false; name.value = ''; message.value = ''; actionId.value = ''; selectedActionIds.value = []; actionSearch.value = ''; contains.value = ''; giftContains.value = ''; viewer.value = ''; delta.value = '';
    actionType.value = 'core.log'; viewerTarget.value = 'specific'; runMode.value = 'all'; milliseconds.value = '1000'; trigger.value = 'tiktok.chat'; cooldown.value = '0'; cooldownScope.value = 'global'; emitTopic.value = ''; emitData.value = '{}'; customTopic.value = ''; fetchUrl.value = ''; fetchMethod.value = 'POST'; fetchHeaders.value = '{}'; fetchBody.value = ''; fetchTimeout.value = '5000'; fetchResponse.value = '2097152'; fetchPrivate.value = false; audioPath.value = ''; audioVolume.value = '1'; audioOverlap.value = 'allow'; scriptSource.value = '';
  }, { flush: 'sync' });

  return {
    name, actionType, message, milliseconds, viewer, viewerTarget,
    delta, trigger, emitTopic, emitData, customTopic, fetchUrl,
    fetchMethod, fetchHeaders, fetchBody, fetchTimeout, fetchResponse, fetchPrivate,
    audioPath, audioVolume, audioOverlap, scriptSource, contains, giftContains,
    actionId, selectedActionIds, runMode, actionSearch, cooldown, cooldownScope,
    loadedSource, loadError, actionIds, actionsValid, actionsReady, matchingActions,
    draft, addAction, removeAction, moveAction, useAction, loadFields,
  };
}
export type ProposalBuilderDraft = ReturnType<typeof useProposalBuilderDraft>;
