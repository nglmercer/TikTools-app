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
  viewer: 'ID exacto del espectador', delta: 'Cambio de puntos', pointsNote: 'Usa un ID literal, sin @ ni variables. El propietario y el anfitrión deben aprobar el efecto y los permisos para ese espectador; este borrador no ajusta puntos.',
  viewerTarget: 'Espectador que recibe el ajuste', specificViewer: 'Un espectador concreto', triggeringViewer: 'Quien origine el evento',
  dynamicPointsNote: 'El ID se tomará de cada evento. El evento debe contener un espectador y sus permisos se comprobarán en cada ejecución. El propietario debe revisar y aprobar la definición; este borrador no ajusta puntos.',
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
  actionDisabled: 'inactiva', actionEnabled: 'activa', actionStatusNote: 'La lista es una instantánea. El anfitrión comprobará las referencias y el propietario revisará la definición antes de activarla.',
  generate: 'Preparar borrador', invalid: 'Completa los campos válidos antes de preparar el borrador.',
  confirm: '¿Reemplazar el texto abierto?', warning: 'Se reemplazará el texto abierto por el borrador mostrado. No se guardará ni enviará todavía. Copia antes cualquier contenido que desees conservar.',
  apply: 'Usar este borrador', changed: 'La propuesta, los campos o el acceso cambiaron. Vuelve a revisar el borrador.', preview: 'Borrador nuevo',
} : {
  title: 'Create a proposal with fields', note: 'Generate a disabled draft. Before sending, save it and inspect the host status.',
  name: 'Name', actionType: 'Action type', log: 'Log a message', delay: 'Wait', points: 'Adjust a viewer’s points', message: 'Message', milliseconds: 'Milliseconds',
  viewer: 'Exact viewer ID', delta: 'Points change', pointsNote: 'Use a literal ID without @ or variables. The owner and host must approve the effect and permissions for that viewer; this draft does not adjust points.',
  viewerTarget: 'Viewer receiving the adjustment', specificViewer: 'A specific viewer', triggeringViewer: 'Viewer who triggers the event',
  dynamicPointsNote: 'The ID comes from each event. The event must contain a viewer, and permissions are checked on every execution. The owner must review and approve the definition; this draft does not adjust points.',
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
  actionDisabled: 'disabled', actionEnabled: 'enabled', actionStatusNote: 'This list is a snapshot. The host checks references and the owner reviews the definition before activation.',
  generate: 'Prepare draft', invalid: 'Complete the valid fields before preparing the draft.',
  confirm: 'Replace the open text?', warning: 'The open text will be replaced by the draft shown. It will not be saved or sent yet. Copy anything you want to keep first.',
  apply: 'Use this draft', changed: 'The proposal, fields, or access changed. Review the draft again.', preview: 'New draft',
});

const name = ref('');
const actionType = ref<'core.log' | 'core.delay' | 'core.points'>('core.log');
const message = ref('');
const milliseconds = ref('1000');
const viewer = ref('');
const viewerTarget = ref<'specific' | 'event'>('specific');
const delta = ref('');
const trigger = ref<'tiktok.chat' | 'tiktok.gift' | 'tiktok.like' | 'tiktok.follow' | 'tiktok.share' | 'tiktok.join'>('tiktok.chat');
const contains = ref('');
const giftContains = ref('');
const actionId = ref('');
const selectedActionIds = ref<string[]>([]);
const runMode = ref<'all' | 'random'>('all');
const actionSearch = ref('');
const cooldown = ref('0');
const cooldownScope = ref<'global' | 'user'>('global');
const encoder = new TextEncoder();
const triggers = new Set(['tiktok.chat', 'tiktok.gift', 'tiktok.like', 'tiktok.follow', 'tiktok.share', 'tiktok.join']);
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
const draft = computed<string | null>(() => {
  const id = props.model.id.value;
  if (!props.model.editable.value || !canonicalId(id) || !name.value.trim() || encoder.encode(name.value).length > 256) return null;
  if (props.model.kind.value === 'action') {
    if (actionType.value === 'core.log') {
      if (!message.value.trim() || encoder.encode(message.value).length > 8192) return null;
      return JSON.stringify({ id, name: name.value, enabled: false, typeId: 'core.log', config: { message: message.value }, schemaVersion: 2 }, null, 2);
    }
    if (actionType.value === 'core.delay') {
      const ms = integer(milliseconds.value, 60_000);
      if (ms === null) return null;
      return JSON.stringify({ id, name: name.value, enabled: false, typeId: 'core.delay', config: { ms }, schemaVersion: 2 }, null, 2);
    }
    if (actionType.value === 'core.points') {
      const amount = pointsChange(delta.value);
      if (amount === null || !['specific', 'event'].includes(viewerTarget.value) || (viewerTarget.value === 'specific' && !literalViewer(viewer.value))) return null;
      const uniqueId = viewerTarget.value === 'event' ? '{{event.user.uniqueId}}' : viewer.value;
      return JSON.stringify({ id, name: name.value, enabled: false, typeId: 'core.points', config: { uniqueId, delta: amount }, schemaVersion: 2 }, null, 2);
    }
    return null;
  }
  const wait = integer(cooldown.value, 86_400_000);
  if (wait === null || !triggers.has(trigger.value) || !actionsValid.value || !['all', 'random'].includes(runMode.value) ||
    !['global', 'user'].includes(cooldownScope.value) ||
    (trigger.value === 'tiktok.chat' && encoder.encode(contains.value).length > 500) ||
    (trigger.value === 'tiktok.gift' && encoder.encode(giftContains.value).length > 500)) return null;
  const filters = trigger.value === 'tiktok.chat' && contains.value
    ? [{ path: 'event.data.comment', operator: 'contains', value: contains.value }]
    : trigger.value === 'tiktok.gift' && giftContains.value
      ? [{ path: 'event.data.giftName', operator: 'contains', value: giftContains.value }]
      : [];
  return JSON.stringify({ id, name: name.value, enabled: false, trigger: trigger.value,
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
}
// Source and form state are both private to the current connection/selection.
watch(() => [props.model.identityKey.value, props.model.kind.value, props.model.id.value], () => {
  capture.value = null; name.value = ''; message.value = ''; actionId.value = ''; selectedActionIds.value = []; actionSearch.value = ''; contains.value = ''; giftContains.value = ''; viewer.value = ''; delta.value = '';
  actionType.value = 'core.log'; viewerTarget.value = 'specific'; runMode.value = 'all'; milliseconds.value = '1000'; trigger.value = 'tiktok.chat'; cooldown.value = '0'; cooldownScope.value = 'global';
}, { flush: 'sync' });
</script>

<template>
  <details class="proposal-builder">
    <summary>{{ labels.title }}</summary>
    <p>{{ labels.note }}</p>
    <div class="fields">
      <label>{{ labels.name }}<input v-model="name" maxlength="256" :disabled="model.busy.value"></label>
      <template v-if="model.kind.value === 'action'">
        <label>{{ labels.actionType }}<select v-model="actionType" :disabled="model.busy.value"><option value="core.log">{{ labels.log }}</option><option value="core.delay">{{ labels.delay }}</option><option value="core.points">{{ labels.points }}</option></select></label>
        <label v-if="actionType === 'core.log'">{{ labels.message }}<input v-model="message" maxlength="8192" :disabled="model.busy.value"></label>
        <label v-else-if="actionType === 'core.delay'">{{ labels.milliseconds }}<input v-model="milliseconds" type="number" min="0" max="60000" step="1" :disabled="model.busy.value"></label>
        <template v-else>
          <label>{{ labels.viewerTarget }}<select v-model="viewerTarget" :disabled="model.busy.value"><option value="specific">{{ labels.specificViewer }}</option><option value="event">{{ labels.triggeringViewer }}</option></select></label>
          <label v-if="viewerTarget === 'specific'">{{ labels.viewer }}<input v-model="viewer" maxlength="128" spellcheck="false" autocomplete="off" :disabled="model.busy.value"></label>
          <label>{{ labels.delta }}<input v-model="delta" type="number" min="-1000000000" max="1000000000" step="any" :disabled="model.busy.value"></label>
        </template>
      </template>
      <template v-else>
        <label>{{ labels.trigger }}<select v-model="trigger" :disabled="model.busy.value"><option value="tiktok.chat">{{ labels.chat }}</option><option value="tiktok.gift">{{ labels.gift }}</option><option value="tiktok.like">{{ labels.like }}</option><option value="tiktok.follow">{{ labels.follow }}</option><option value="tiktok.share">{{ labels.share }}</option><option value="tiktok.join">{{ labels.join }}</option></select></label>
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
input, select, button { padding: 8px; color: inherit; background: var(--color-bg-panel); border: 1px solid var(--color-border-subtle); border-radius: 6px; }
button:disabled { opacity: .5; } pre { max-height: 260px; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; }
.available-actions { margin-top:12px; } .available-actions ul { max-height:14rem; overflow:auto; padding-left:1.5rem; }
.available-actions li { margin:6px 0; overflow-wrap:anywhere; } .available-actions li button { margin-left:8px; }
</style>
