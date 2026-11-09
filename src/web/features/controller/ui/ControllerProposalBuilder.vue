<script setup lang="ts">
import { computed, ref, unref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ProposalConfirmation, ProposalModel } from '../model/proposals.ts';
import type { AutomationModel, AutomationRow } from '../model/automations.ts';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';

const props = defineProps<{ model: ProposalModel; automation?: AutomationModel }>();
const { locale } = useI18n();
const labels = computed(() => locale.value === 'es' ? {
  title: 'Crear una propuesta con campos', note: 'Genera un borrador inactivo. Antes de enviarlo tendrás que guardarlo y revisar el estado del anfitrión.',
  name: 'Nombre', actionType: 'Tipo de acción', log: 'Registrar un mensaje', delay: 'Esperar', points: 'Ajustar puntos de un espectador', message: 'Mensaje', milliseconds: 'Milisegundos',
  emit: 'Emitir evento interno', topic: 'Nombre exacto del evento', emitData: 'Datos del evento (objeto JSON)', emitHint: 'Usa hasta 64 letras minúsculas, números, puntos, guiones o guiones bajos. El anfitrión debe permitir el nombre exacto y, para campos públicos del directo, la lectura de LIVE.', custom: 'Evento interno', customHint: 'Este evento solo recibe eventos internos autorizados; no se inicia desde TikTok directamente. Usa un nombre exacto en minúsculas.',
  fetch: 'Enviar solicitud HTTP', fetchUrl: 'URL de destino', fetchMethod: 'Método', fetchHeaders: 'Cabeceras (objeto JSON de texto)', fetchBody: 'Cuerpo de la solicitud', fetchTimeout: 'Tiempo límite (ms)', fetchResponse: 'Límite de respuesta (bytes)', fetchPrivate: 'Permitir red privada', fetchHint: 'El anfitrión debe autorizar el destino, método, cabeceras, cuerpo y límites. Las respuestas no se comparten como eventos.',
  viewer: 'ID exacto del espectador', delta: 'Cambio de puntos', pointsNote: 'Usa un ID literal, sin @ ni variables. El anfitrión comprueba los permisos y límites concedidos para ese espectador; este borrador no ajusta puntos.',
  viewerTarget: 'Espectador que recibe el ajuste', specificViewer: 'Un espectador concreto', triggeringViewer: 'Quien origine el evento',
  dynamicPointsNote: 'El ID se tomará de cada evento. El evento debe contener un espectador y sus permisos se comprobarán en cada ejecución. La autorización debe permitir destinos dinámicos; este borrador no ajusta puntos.',
  trigger: 'Cuándo ocurre', chat: 'Mensaje en el chat', gift: 'Regalo', like: 'Me gusta', follow: 'Seguir', share: 'Compartir', join: 'Ingreso',
  contains: 'El mensaje contiene (opcional)', actionId: 'ID de acción (opcional)', cooldown: 'Espera entre ejecuciones (ms)',
  giftContains: 'El nombre del regalo contiene (opcional)', giftHint: 'La coincidencia es parcial. Si el nombre no está disponible, el anfitrión puede mostrar «Gift».',
  cooldownScope: 'A quién se aplica la espera', cooldownGlobal: 'Compartida para este evento', cooldownViewer: 'Por espectador para este evento',
  cooldownNote: 'La espera puede reiniciarse al reiniciar la app o bajo carga muy alta. Los eventos sin ID de espectador comparten una espera.',
  actionNote: 'El ID escrito se incluirá al preparar el borrador. Usa Agregar para conservarlo antes de elegir otro. El anfitrión comprobará el acceso; sin acciones vinculadas, el evento no hará nada.',
  addAction: 'Agregar acción', selectedActions: 'Acciones agregadas en orden', removeAction: (id: string) => `Quitar ${id}`,
  moveEarlier: (id: string) => `Mover ${id} hacia arriba`, moveLater: (id: string) => `Mover ${id} hacia abajo`,
  actionError: 'Usa IDs válidos, sin repetir, y agrega como máximo 512 acciones.',
  runMode: 'Cómo ejecutar las acciones', runAll: 'Todas las acciones activas, en orden', runRandom: 'Una acción activa al azar',
  availableActions: 'Acciones visibles en el anfitrión', actionSearch: 'Buscar acción visible', noActions: 'No hay acciones visibles que coincidan.',
  actionLimit: 'Se muestran las primeras 50 coincidencias. Busca por nombre o ID para encontrar otras.', selectAction: 'Usar ID',
  actionDisabled: 'inactiva', actionEnabled: 'activa', actionStatusNote: 'La lista es una instantánea. El anfitrión comprobará las referencias y los permisos al activar la definición.',
  generate: 'Preparar borrador', invalid: 'Completa los campos válidos antes de preparar el borrador.',
  loadFields: 'Cargar campos del texto abierto', loadFieldsHint: 'Carga acciones de registro, espera, puntos, HTTP o eventos internos y eventos compatibles. Conserva los demás campos; al cambiar el tipo de acción, se reemplaza su configuración.', loadFieldsError: 'No se pueden cargar estos campos. Revisa el JSON o edítalo directamente.', staleFields: 'El texto abierto cambió. Vuelve a cargar los campos antes de preparar el borrador.',
  confirm: '¿Reemplazar el texto abierto?', warning: 'Se reemplazará el texto abierto por el borrador mostrado. No se guardará ni enviará todavía. Copia antes cualquier contenido que desees conservar.',
  apply: 'Usar este borrador', changed: 'La propuesta, los campos o el acceso cambiaron. Vuelve a revisar el borrador.', preview: 'Borrador nuevo',
} : {
  title: 'Create a proposal with fields', note: 'Generate a disabled draft. Before sending, save it and inspect the host status.',
  name: 'Name', actionType: 'Action type', log: 'Log a message', delay: 'Wait', points: 'Adjust a viewer’s points', message: 'Message', milliseconds: 'Milliseconds',
  emit: 'Emit an internal event', topic: 'Exact event name', emitData: 'Event data (JSON object)', emitHint: 'Use up to 64 lowercase letters, digits, dots, hyphens or underscores. The host must allow the exact name and, for public LIVE fields, LIVE read access.', custom: 'Internal event', customHint: 'This event only receives authorized internal events; TikTok does not start it directly. Use an exact lowercase name.',
  fetch: 'Send an HTTP request', fetchUrl: 'Destination URL', fetchMethod: 'Method', fetchHeaders: 'Headers (JSON object of strings)', fetchBody: 'Request body', fetchTimeout: 'Timeout (ms)', fetchResponse: 'Response limit (bytes)', fetchPrivate: 'Allow private network', fetchHint: 'The host must authorize the destination, method, headers, body and limits. Responses are not emitted as events.',
  viewer: 'Exact viewer ID', delta: 'Points change', pointsNote: 'Use a literal ID without @ or variables. The host checks the granted permissions and limits for that viewer; this draft does not adjust points.',
  viewerTarget: 'Viewer receiving the adjustment', specificViewer: 'A specific viewer', triggeringViewer: 'Viewer who triggers the event',
  dynamicPointsNote: 'The ID comes from each event. The event must contain a viewer, and permissions are checked on every execution. The authorization must allow dynamic targets; this draft does not adjust points.',
  trigger: 'When it happens', chat: 'Chat message', gift: 'Gift', like: 'Like', follow: 'Follow', share: 'Share', join: 'Join',
  contains: 'Message contains (optional)', actionId: 'Action ID (optional)', cooldown: 'Delay between runs (ms)',
  giftContains: 'Gift name contains (optional)', giftHint: 'This is a partial match. If the name is unavailable, the host may display “Gift”.',
  cooldownScope: 'Who the delay applies to', cooldownGlobal: 'Shared for this event', cooldownViewer: 'Per viewer for this event',
  cooldownNote: 'The delay can reset after app restart or under very high load. Events without a viewer ID share a delay.',
  actionNote: 'The typed ID is included when preparing the draft. Use Add to keep it before choosing another. The host checks access; without linked actions, the event will do nothing.',
  addAction: 'Add action', selectedActions: 'Added actions in order', removeAction: (id: string) => `Remove ${id}`,
  moveEarlier: (id: string) => `Move ${id} up`, moveLater: (id: string) => `Move ${id} down`,
  actionError: 'Use valid, unique IDs and add at most 512 actions.',
  runMode: 'How to run the actions', runAll: 'All enabled actions, in order', runRandom: 'One enabled action at random',
  availableActions: 'Actions visible on the host', actionSearch: 'Search visible actions', noActions: 'No visible actions match.',
  actionLimit: 'Showing the first 50 matches. Search by name or ID to find others.', selectAction: 'Use ID',
  actionDisabled: 'disabled', actionEnabled: 'enabled', actionStatusNote: 'This list is a snapshot. The host checks references and permissions when activating the definition.',
  generate: 'Prepare draft', invalid: 'Complete the valid fields before preparing the draft.',
  loadFields: 'Load fields from open text', loadFieldsHint: 'Loads supported log, delay, points, HTTP, emit and event definitions. Other fields are preserved; changing the action type replaces its configuration.', loadFieldsError: 'These fields cannot be loaded. Check the JSON or edit it directly.', staleFields: 'The open text changed. Reload the fields before preparing the draft.',
  confirm: 'Replace the open text?', warning: 'The open text will be replaced by the draft shown. It will not be saved or sent yet. Copy anything you want to keep first.',
  apply: 'Use this draft', changed: 'The proposal, fields, or access changed. Review the draft again.', preview: 'New draft',
});

const name = ref('');
const actionType = ref<'core.log' | 'core.delay' | 'core.points' | 'core.emit' | 'core.fetch'>('core.log');
const message = ref('');
const milliseconds = ref('1000');
const viewer = ref('');
const viewerTarget = ref<'specific' | 'event'>('specific');
const delta = ref('');
const trigger = ref<'tiktok.chat' | 'tiktok.gift' | 'tiktok.like' | 'tiktok.follow' | 'tiktok.share' | 'tiktok.join' | 'custom'>('tiktok.chat');
const emitTopic = ref(''); const emitData = ref('{}'); const customTopic = ref('');
const fetchUrl = ref(''); const fetchMethod = ref<'GET' | 'POST' | 'PUT' | 'DELETE'>('POST');
const fetchHeaders = ref('{}'); const fetchBody = ref(''); const fetchTimeout = ref('5000'); const fetchResponse = ref('2097152'); const fetchPrivate = ref(false);
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
  actionType.value = 'core.log'; viewerTarget.value = 'specific'; runMode.value = 'all'; milliseconds.value = '1000'; trigger.value = 'tiktok.chat'; cooldown.value = '0'; cooldownScope.value = 'global'; emitTopic.value = ''; emitData.value = '{}'; customTopic.value = ''; fetchUrl.value = ''; fetchMethod.value = 'POST'; fetchHeaders.value = '{}'; fetchBody.value = ''; fetchTimeout.value = '5000'; fetchResponse.value = '2097152'; fetchPrivate.value = false;
}, { flush: 'sync' });
</script>

<template>
  <details class="proposal-builder">
    <summary>{{ labels.title }}</summary>
    <p>{{ labels.note }}</p>
    <p>{{ labels.loadFieldsHint }}</p>
    <button type="button" :disabled="model.busy.value || !model.editable.value" @click="loadFields">{{ labels.loadFields }}</button>
    <p v-if="loadError" role="alert">{{ labels.loadFieldsError }}</p>
    <p v-if="loadedSource && loadedSource.text !== model.text.value" role="status">{{ labels.staleFields }}</p>
    <div class="fields">
      <label>{{ labels.name }}<input v-model="name" maxlength="256" :disabled="model.busy.value"></label>
      <template v-if="model.kind.value === 'action'">
        <label>{{ labels.actionType }}<select v-model="actionType" :disabled="model.busy.value"><option value="core.log">{{ labels.log }}</option><option value="core.delay">{{ labels.delay }}</option><option value="core.points">{{ labels.points }}</option><option value="core.emit">{{ labels.emit }}</option><option value="core.fetch">{{ labels.fetch }}</option></select></label>
        <label v-if="actionType === 'core.log'">{{ labels.message }}<input v-model="message" maxlength="8192" :disabled="model.busy.value"></label>
        <label v-else-if="actionType === 'core.delay'">{{ labels.milliseconds }}<input v-model="milliseconds" type="number" min="0" max="60000" step="1" :disabled="model.busy.value"></label>
        <template v-else-if="actionType === 'core.emit'">
          <label>{{ labels.topic }}<input v-model="emitTopic" maxlength="64" spellcheck="false" :disabled="model.busy.value"></label>
          <label>{{ labels.emitData }}<textarea v-model="emitData" rows="4" spellcheck="false" :disabled="model.busy.value" /></label>
        </template>
        <template v-else-if="actionType === 'core.fetch'">
          <label>{{ labels.fetchUrl }}<input v-model="fetchUrl" maxlength="8192" spellcheck="false" autocomplete="off" :disabled="model.busy.value"></label>
          <label>{{ labels.fetchMethod }}<select v-model="fetchMethod" :disabled="model.busy.value"><option>GET</option><option>POST</option><option>PUT</option><option>DELETE</option></select></label>
          <label>{{ labels.fetchHeaders }}<textarea v-model="fetchHeaders" rows="4" spellcheck="false" autocomplete="off" :disabled="model.busy.value" /></label>
          <label>{{ labels.fetchBody }}<textarea v-model="fetchBody" rows="4" spellcheck="false" autocomplete="off" :disabled="model.busy.value" /></label>
          <label>{{ labels.fetchTimeout }}<input v-model="fetchTimeout" type="number" min="100" max="120000" step="1" :disabled="model.busy.value"></label>
          <label>{{ labels.fetchResponse }}<input v-model="fetchResponse" type="number" min="1" max="2097152" step="1" :disabled="model.busy.value"></label>
          <label><input v-model="fetchPrivate" type="checkbox" :disabled="model.busy.value">{{ labels.fetchPrivate }}</label>
        </template>
        <template v-else>
          <label>{{ labels.viewerTarget }}<select v-model="viewerTarget" :disabled="model.busy.value"><option value="specific">{{ labels.specificViewer }}</option><option value="event">{{ labels.triggeringViewer }}</option></select></label>
          <label v-if="viewerTarget === 'specific'">{{ labels.viewer }}<input v-model="viewer" maxlength="128" spellcheck="false" autocomplete="off" :disabled="model.busy.value"></label>
          <label>{{ labels.delta }}<input v-model="delta" type="number" min="-1000000000" max="1000000000" step="any" :disabled="model.busy.value"></label>
        </template>
      </template>
      <template v-else>
        <label>{{ labels.trigger }}<select v-model="trigger" :disabled="model.busy.value"><option value="tiktok.chat">{{ labels.chat }}</option><option value="tiktok.gift">{{ labels.gift }}</option><option value="tiktok.like">{{ labels.like }}</option><option value="tiktok.follow">{{ labels.follow }}</option><option value="tiktok.share">{{ labels.share }}</option><option value="tiktok.join">{{ labels.join }}</option><option value="custom">{{ labels.custom }}</option></select></label>
        <label v-if="trigger === 'custom'">{{ labels.topic }}<input v-model="customTopic" maxlength="64" spellcheck="false" :disabled="model.busy.value"></label>
        <label v-if="trigger === 'tiktok.chat'">{{ labels.contains }}<input v-model="contains" maxlength="500" :disabled="model.busy.value"></label>
        <label v-if="trigger === 'tiktok.gift'">{{ labels.giftContains }}<input v-model="giftContains" maxlength="500" :disabled="model.busy.value"></label>
        <label>{{ labels.actionId }}<input v-model="actionId" maxlength="128" spellcheck="false" :disabled="model.busy.value"></label>
        <label>{{ labels.runMode }}<select v-model="runMode" :disabled="model.busy.value"><option value="all">{{ labels.runAll }}</option><option value="random">{{ labels.runRandom }}</option></select></label>
        <label>{{ labels.cooldown }}<input v-model="cooldown" type="number" min="0" max="86400000" step="1" :disabled="model.busy.value"></label>
        <label>{{ labels.cooldownScope }}<select v-model="cooldownScope" :disabled="model.busy.value"><option value="global">{{ labels.cooldownGlobal }}</option><option value="user">{{ labels.cooldownViewer }}</option></select></label>
      </template>
    </div>
    <div v-if="model.kind.value === 'event'" class="selected-actions">
      <button type="button" :disabled="model.busy.value || !model.editable.value || !actionId || !actionsValid" @click="addAction">{{ labels.addAction }}</button>
      <p v-if="!actionsValid" role="status">{{ labels.actionError }}</p>
      <template v-if="selectedActionIds.length">
        <h4>{{ labels.selectedActions }}</h4>
        <ol>
          <li v-for="(id, index) in selectedActionIds" :key="id">
            <span>{{ id }}</span>
            <button type="button" :aria-label="labels.moveEarlier(id)" :disabled="model.busy.value || !model.editable.value || index === 0" @click="moveAction(index, -1)">↑</button>
            <button type="button" :aria-label="labels.moveLater(id)" :disabled="model.busy.value || !model.editable.value || index === selectedActionIds.length - 1" @click="moveAction(index, 1)">↓</button>
            <button type="button" :aria-label="labels.removeAction(id)" :disabled="model.busy.value || !model.editable.value" @click="removeAction(index)">×</button>
          </li>
        </ol>
      </template>
    </div>
    <section v-if="model.kind.value === 'event' && actionsReady" :aria-label="labels.availableActions" class="available-actions">
      <h4>{{ labels.availableActions }}</h4>
      <p>{{ labels.actionStatusNote }}</p>
      <label>{{ labels.actionSearch }}<input v-model="actionSearch" type="search" maxlength="128" autocomplete="off" :disabled="model.busy.value"></label>
      <p v-if="!matchingActions.length">{{ labels.noActions }}</p>
      <ul v-else>
        <li v-for="row in matchingActions.slice(0, 50)" :key="row.summary.id">
          <span>{{ row.summary.name }} · {{ row.summary.id }} · {{ row.summary.enabled ? labels.actionEnabled : labels.actionDisabled }}</span>
          <button type="button" :disabled="model.busy.value || !model.editable.value" @click="useAction(row)">{{ labels.selectAction }}</button>
        </li>
      </ul>
      <p v-if="matchingActions.length > 50">{{ labels.actionLimit }}</p>
    </section>
    <p v-if="model.kind.value === 'event'">{{ labels.actionNote }}</p>
    <p v-if="model.kind.value === 'event' && trigger === 'tiktok.gift'">{{ labels.giftHint }}</p>
    <p v-if="model.kind.value === 'event' && trigger === 'custom'">{{ labels.customHint }}</p>
    <p v-if="model.kind.value === 'action' && actionType === 'core.emit'">{{ labels.emitHint }}</p>
    <p v-if="model.kind.value === 'action' && actionType === 'core.fetch'">{{ labels.fetchHint }}</p>
    <p v-if="model.kind.value === 'event' && cooldown !== '0'">{{ labels.cooldownNote }}</p>
    <p v-if="model.kind.value === 'action' && actionType === 'core.points'">{{ viewerTarget === 'event' ? labels.dynamicPointsNote : labels.pointsNote }}</p>
    <p v-if="draft === null" role="status">{{ labels.invalid }}</p>
    <button type="button" :disabled="draft === null || !model.prepare('save')" @click="prepare">{{ labels.generate }}</button>
  </details>
  <BaseConfirmDialog v-model:open="open" :title="labels.confirm" :message="labels.warning" :confirm-label="labels.apply" :busy="!current" @confirm="apply">
    <template v-if="capture">
      <p>{{ labels.preview }}</p><pre>{{ capture.generated }}</pre>
      <p v-if="!current" role="status">{{ labels.changed }}</p>
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
