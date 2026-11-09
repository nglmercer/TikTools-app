<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ProposalConfirmation, ProposalModel } from '../model/proposals.ts';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';

const props = defineProps<{ model: ProposalModel }>();
const { locale } = useI18n();
const labels = computed(() => locale.value === 'es' ? {
  title: 'Crear una propuesta con campos', note: 'Genera un borrador inactivo. Antes de enviarlo tendrás que guardarlo y revisar el estado del anfitrión.',
  name: 'Nombre', actionType: 'Tipo de acción', log: 'Registrar un mensaje', delay: 'Esperar', points: 'Ajustar puntos de un espectador', message: 'Mensaje', milliseconds: 'Milisegundos',
  viewer: 'ID exacto del espectador', delta: 'Cambio de puntos', pointsNote: 'Usa un ID literal, sin @ ni variables. El propietario y el anfitrión deben aprobar el efecto y los permisos para ese espectador; este borrador no ajusta puntos.',
  trigger: 'Cuándo ocurre', chat: 'Mensaje en el chat', gift: 'Regalo', like: 'Me gusta', follow: 'Seguir', share: 'Compartir', join: 'Ingreso',
  contains: 'El mensaje contiene (opcional)', actionId: 'ID de acción (opcional)', cooldown: 'Espera entre ejecuciones (ms)',
  actionNote: 'El anfitrión verificará el acceso a la acción. Sin una acción vinculada, el evento no hará nada.',
  generate: 'Preparar borrador', invalid: 'Completa los campos válidos antes de preparar el borrador.',
  confirm: '¿Reemplazar el texto abierto?', warning: 'Se reemplazará el texto abierto por el borrador mostrado. No se guardará ni enviará todavía. Copia antes cualquier contenido que desees conservar.',
  apply: 'Usar este borrador', changed: 'La propuesta, los campos o el acceso cambiaron. Vuelve a revisar el borrador.', preview: 'Borrador nuevo',
} : {
  title: 'Create a proposal with fields', note: 'Generate a disabled draft. Before sending, save it and inspect the host status.',
  name: 'Name', actionType: 'Action type', log: 'Log a message', delay: 'Wait', points: 'Adjust a viewer’s points', message: 'Message', milliseconds: 'Milliseconds',
  viewer: 'Exact viewer ID', delta: 'Points change', pointsNote: 'Use a literal ID without @ or variables. The owner and host must approve the effect and permissions for that viewer; this draft does not adjust points.',
  trigger: 'When it happens', chat: 'Chat message', gift: 'Gift', like: 'Like', follow: 'Follow', share: 'Share', join: 'Join',
  contains: 'Message contains (optional)', actionId: 'Action ID (optional)', cooldown: 'Delay between runs (ms)',
  actionNote: 'The host checks access to the action. Without a linked action, the event will do nothing.',
  generate: 'Prepare draft', invalid: 'Complete the valid fields before preparing the draft.',
  confirm: 'Replace the open text?', warning: 'The open text will be replaced by the draft shown. It will not be saved or sent yet. Copy anything you want to keep first.',
  apply: 'Use this draft', changed: 'The proposal, fields, or access changed. Review the draft again.', preview: 'New draft',
});

const name = ref('');
const actionType = ref<'core.log' | 'core.delay' | 'core.points'>('core.log');
const message = ref('');
const milliseconds = ref('1000');
const viewer = ref('');
const delta = ref('');
const trigger = ref<'tiktok.chat' | 'tiktok.gift' | 'tiktok.like' | 'tiktok.follow' | 'tiktok.share' | 'tiktok.join'>('tiktok.chat');
const contains = ref('');
const actionId = ref('');
const cooldown = ref('0');
const encoder = new TextEncoder();
const triggers = new Set(['tiktok.chat', 'tiktok.gift', 'tiktok.like', 'tiktok.follow', 'tiktok.share', 'tiktok.join']);
const integer = (value: string, max: number): number | null => {
  if (!/^(0|[1-9]\d*)$/.test(value)) return null;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed <= max ? parsed : null;
};
const canonicalId = (value: string) => value.length > 0 && value.trim() === value
  && encoder.encode(value).length <= 128 && !/[\p{Cc}*]/u.test(value);
const literalViewer = (value: string) => canonicalId(value) && !value.startsWith('@') && !value.includes('{{') && !value.includes('}}');
const pointsChange = (value: string): number | null => {
  if (!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(value)) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed !== 0 && Math.abs(parsed) <= 1_000_000_000 ? parsed : null;
};
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
      if (!literalViewer(viewer.value) || amount === null) return null;
      return JSON.stringify({ id, name: name.value, enabled: false, typeId: 'core.points', config: { uniqueId: viewer.value, delta: amount }, schemaVersion: 2 }, null, 2);
    }
    return null;
  }
  const wait = integer(cooldown.value, 86_400_000);
  if (wait === null || !triggers.has(trigger.value) || (actionId.value && !canonicalId(actionId.value)) || encoder.encode(contains.value).length > 500) return null;
  const filters = trigger.value === 'tiktok.chat' && contains.value
    ? [{ path: 'event.data.comment', operator: 'contains', value: contains.value }]
    : [];
  return JSON.stringify({ id, name: name.value, enabled: false, trigger: trigger.value,
    actionIds: actionId.value ? [actionId.value] : [], filters, cooldownMs: wait, cooldownScope: 'global', runMode: 'all', schemaVersion: 1 }, null, 2);
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
  capture.value = null; name.value = ''; message.value = ''; actionId.value = ''; contains.value = ''; viewer.value = ''; delta.value = '';
  actionType.value = 'core.log'; milliseconds.value = '1000'; trigger.value = 'tiktok.chat'; cooldown.value = '0';
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
          <label>{{ labels.viewer }}<input v-model="viewer" maxlength="128" spellcheck="false" autocomplete="off" :disabled="model.busy.value"></label>
          <label>{{ labels.delta }}<input v-model="delta" type="number" min="-1000000000" max="1000000000" step="any" :disabled="model.busy.value"></label>
        </template>
      </template>
      <template v-else>
        <label>{{ labels.trigger }}<select v-model="trigger" :disabled="model.busy.value"><option value="tiktok.chat">{{ labels.chat }}</option><option value="tiktok.gift">{{ labels.gift }}</option><option value="tiktok.like">{{ labels.like }}</option><option value="tiktok.follow">{{ labels.follow }}</option><option value="tiktok.share">{{ labels.share }}</option><option value="tiktok.join">{{ labels.join }}</option></select></label>
        <label v-if="trigger === 'tiktok.chat'">{{ labels.contains }}<input v-model="contains" maxlength="500" :disabled="model.busy.value"></label>
        <label>{{ labels.actionId }}<input v-model="actionId" maxlength="128" spellcheck="false" :disabled="model.busy.value"></label>
        <label>{{ labels.cooldown }}<input v-model="cooldown" type="number" min="0" max="86400000" step="1" :disabled="model.busy.value"></label>
      </template>
    </div>
    <p v-if="model.kind.value === 'event'">{{ labels.actionNote }}</p>
    <p v-if="model.kind.value === 'action' && actionType === 'core.points'">{{ labels.pointsNote }}</p>
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
</style>
