<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ProposalModel, ProposalConfirmation } from '../model/proposals.ts';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
const props = defineProps<{ model: ProposalModel }>();
const { locale } = useI18n();
const family = ref<'action' | 'event'>('action'); const targetId = ref('');
const dialog = ref<ProposalConfirmation | null>(null);
const open = computed({ get: () => dialog.value !== null, set: (value: boolean) => { if (!value) dialog.value = null; } });
const current = computed(() => dialog.value !== null && props.model.current(dialog.value));
const labels = computed(() => locale.value === 'es' ? {
  title: 'Propuestas de automatización', note: 'Envía una definición inactiva para que el propietario la revise. El anfitrión solo devuelve un resumen; el texto editable es tu copia local.',
  family: 'Tipo de propuesta', action: 'Acción', event: 'Evento', id: 'ID de la propuesta', start: 'Preparar propuesta nueva', stored: 'Copias guardadas en esta PC',
  load: 'Abrir copia guardada', inspect: 'Consultar estado actual', missing: 'No existe una definición actual para este ID.', present: 'Existe una definición actual. Solo su autor puede proponer cambios; el anfitrión comprobará el acceso.',
  base: 'Revisión base', mode: 'Operación', create: 'Crear', update: 'Reemplazar', useBase: 'Usar este estado como base', json: 'JSON de tu propuesta',
  save: 'Guardar propuesta en esta PC', send: 'Enviar propuesta para revisión', remove: 'Eliminar copia guardada', unsaved: 'Este texto tiene cambios sin guardar.', matches: 'El texto coincide con la copia guardada.',
  saveTitle: '¿Guardar la propuesta en esta PC?', saveNote: 'Se conservará el texto completo, incluidos los valores sensibles que contenga, en el perfil privado de este controlador.',
  sendTitle: '¿Enviar esta propuesta al anfitrión?', sendNote: 'Se enviará una sola vez la copia guardada que se muestra. Debe estar inactiva. El propietario deberá revisar y adoptar la definición antes de ejecutarla.',
  removeTitle: '¿Eliminar la copia guardada?', removeNote: 'Se eliminará el texto guardado en esta PC. El texto abierto se conserva hasta cerrar o cambiar de perfil.',
  baseTitle: '¿Cambiar la base de reemplazo?', baseNote: 'La revisión consultada será la nueva base de esta propuesta. Compara su estado, revisa tu texto y guarda esta elección antes de enviar.',
  changed: 'La propuesta o el acceso cambió. Cierra la confirmación y vuelve a revisarla.', limit: 'Máximo: 256 KiB. El JSON debe incluir el mismo ID y enabled: false.', retry: 'Actualizar copias guardadas',
} : {
  title: 'Automation proposals', note: 'Submit a disabled definition for owner review. The host returns only a summary; the editable text is your local copy.',
  family: 'Proposal type', action: 'Action', event: 'Event', id: 'Proposal ID', start: 'Prepare new proposal', stored: 'Copies saved on this PC',
  load: 'Open saved copy', inspect: 'Inspect current status', missing: 'No current definition exists for this ID.', present: 'A current definition exists. Only its author can propose changes; the host will check access.',
  base: 'Base revision', mode: 'Operation', create: 'Create', update: 'Replace', useBase: 'Use this status as base', json: 'Your proposal JSON',
  save: 'Save proposal on this PC', send: 'Send proposal for review', remove: 'Remove saved copy', unsaved: 'This text has unsaved changes.', matches: 'The text matches the saved copy.',
  saveTitle: 'Save this proposal on this PC?', saveNote: 'The complete text, including any sensitive values it contains, will be retained in this controller’s private profile.',
  sendTitle: 'Send this proposal to the host?', sendNote: 'The saved copy shown is sent once. It must be disabled. The owner must review and adopt the definition before it can run.',
  removeTitle: 'Remove the saved copy?', removeNote: 'The text saved on this PC will be removed. The open text stays until you close or change the profile.',
  baseTitle: 'Change the replacement base?', baseNote: 'The inspected revision becomes this proposal’s new base. Compare its status, review your text and save this choice before sending.',
  changed: 'The proposal or access changed. Close this confirmation and review it again.', limit: 'Maximum: 256 KiB. JSON must include the same ID and enabled: false.', retry: 'Refresh saved copies',
});
function choose(): void { props.model.select(family.value, targetId.value); }
async function load(kind: 'action' | 'event', id: string): Promise<void> { if (props.model.kind.value !== kind || props.model.id.value !== id) props.model.select(kind, id); await props.model.load(); }
function request(operation: ProposalConfirmation['operation']): void { dialog.value = props.model.prepare(operation); }
async function confirm(): Promise<void> { const capture = dialog.value; if (!capture || !current.value) return; dialog.value = null; await props.model.confirm(capture); }
watch(() => props.model.identityKey.value, () => { dialog.value = null; targetId.value = ''; });
const dialogTitle = computed(() => dialog.value?.operation === 'send' ? labels.value.sendTitle : dialog.value?.operation === 'remove' ? labels.value.removeTitle : dialog.value?.operation === 'base' ? labels.value.baseTitle : labels.value.saveTitle);
const dialogNote = computed(() => dialog.value?.operation === 'send' ? labels.value.sendNote : dialog.value?.operation === 'remove' ? labels.value.removeNote : dialog.value?.operation === 'base' ? labels.value.baseNote : labels.value.saveNote);
const dialogLabel = computed(() => dialog.value?.operation === 'send' ? labels.value.send : dialog.value?.operation === 'remove' ? labels.value.remove : dialog.value?.operation === 'base' ? labels.value.useBase : labels.value.save);
</script>

<template>
  <section
    v-if="model.allowed.value"
    class="proposals"
    :aria-label="labels.title"
  >
    <h2>{{ labels.title }}</h2><p>{{ labels.note }}</p>
    <p
      v-if="model.error.value"
      role="alert"
    >
      {{ model.error.value }}
    </p>
    <button
      :disabled="model.busy.value"
      @click="model.reloadInventory()"
    >
      {{ labels.retry }}
    </button>
    <form
      autocomplete="off"
      @submit.prevent="choose"
    >
      <label>{{ labels.family }}<select
        v-model="family"
        :disabled="model.busy.value"
      ><option value="action">{{ labels.action }}</option><option value="event">{{ labels.event }}</option></select></label>
      <label>{{ labels.id }}<input
        v-model="targetId"
        maxlength="128"
        spellcheck="false"
        :disabled="model.busy.value"
      ></label>
      <button :disabled="model.busy.value || !targetId.trim()">
        {{ labels.start }}
      </button>
    </form>
    <details v-if="model.rows.value.length">
      <summary>{{ labels.stored }}</summary><ul>
        <li
          v-for="item in model.rows.value"
          :key="`${item.kind}:${item.id}`"
        >
          <span>{{ item.kind === 'action' ? labels.action : labels.event }} · {{ item.id }} · {{ labels.base }} {{ item.baseRevision }}</span>
          <button
            :disabled="model.busy.value"
            @click="load(item.kind, item.id)"
          >
            {{ labels.load }}
          </button>
        </li>
      </ul>
    </details>
    <template v-if="model.editable.value">
      <h3>{{ model.kind.value === 'action' ? labels.action : labels.event }} · {{ model.id.value }}</h3>
      <p>{{ labels.mode }}: {{ model.mode.value === 'create' ? labels.create : labels.update }} · {{ labels.base }} {{ model.baseRevision.value }}</p>
      <div class="actions">
        <button
          :disabled="model.busy.value"
          @click="model.load()"
        >
          {{ labels.load }}
        </button><button
          :disabled="model.busy.value"
          @click="model.inspect()"
        >
          {{ labels.inspect }}
        </button>
      </div>
      <div
        v-if="model.inspection.value"
        class="status"
      >
        <p>{{ model.inspection.value.summary ? labels.present : labels.missing }}</p>
        <p>{{ model.inspection.value.summary?.name }} · {{ labels.base }} {{ model.inspection.value.revision }}</p>
        <button
          :disabled="!model.prepare('base')"
          @click="request('base')"
        >
          {{ labels.useBase }}
        </button>
      </div>
      <label>{{ labels.json }}<textarea
        :value="model.text.value"
        rows="14"
        spellcheck="false"
        autocomplete="off"
        :disabled="model.busy.value"
        @input="model.edit(($event.target as HTMLTextAreaElement).value)"
      /></label>
      <p>{{ labels.limit }}</p><p role="status">
        {{ model.matches.value ? labels.matches : labels.unsaved }}
      </p>
      <div class="actions">
        <button
          :disabled="!model.prepare('save')"
          @click="request('save')"
        >
          {{ labels.save }}
        </button>
        <button
          :disabled="!model.prepare('remove')"
          @click="request('remove')"
        >
          {{ labels.remove }}
        </button>
        <button
          :disabled="!model.canSend.value"
          @click="request('send')"
        >
          {{ labels.send }}
        </button>
      </div>
    </template>
  </section>
  <BaseConfirmDialog
    v-model:open="open"
    :title="dialogTitle"
    :message="dialogNote"
    :confirm-label="dialogLabel"
    :busy="!current || model.busy.value"
    @confirm="confirm"
  >
    <template v-if="dialog">
      <code>{{ dialog.origin }}</code><code>Host {{ dialog.hostId }}</code>
      <p>{{ dialog.kind === 'action' ? labels.action : labels.event }} · {{ dialog.id }} · {{ dialog.mode === 'create' ? labels.create : labels.update }} · {{ labels.base }} {{ dialog.baseRevision }}</p>
      <p v-if="dialog.operation === 'base'">
        {{ labels.base }} → {{ model.inspection.value?.revision }}
      </p>
      <pre v-if="dialog.operation !== 'remove'">{{ dialog.text }}</pre>
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
.proposals { padding:20px; margin-top:18px; border:1px solid var(--color-border-subtle); border-radius:8px; overflow-wrap:anywhere; }
form,.actions,li { display:flex; gap:12px; flex-wrap:wrap; margin:12px 0; align-items:end; } label { display:flex; flex-direction:column; gap:6px; flex:1; min-width:0; }
input,select,textarea,button { padding:8px; color:inherit; background:var(--color-bg-panel); border:1px solid var(--color-border-subtle); border-radius:6px; }
textarea { width:100%; box-sizing:border-box; font-family:monospace; } button { cursor:pointer; } button:disabled { opacity:.5; cursor:default; }
pre { max-height:260px; overflow:auto; white-space:pre-wrap; overflow-wrap:anywhere; } code { display:block; overflow-wrap:anywhere; } p { margin:10px 0; } h2 { font-size:18px; }
@media(max-width:600px) { form { align-items:stretch; flex-direction:column; } }
</style>
