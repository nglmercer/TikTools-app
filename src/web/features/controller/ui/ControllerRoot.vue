<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { createControllerModel, type CommandTarget, type PointsTarget, type PendingCommand, type ProfileForgetTarget, type RemoteProfile, type ReviewTarget, type WorkflowOperation } from '../model/controller.ts';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import ControllerAutomations from './ControllerAutomations.vue';
import ControllerProposals from './ControllerProposals.vue';
import ControllerLiveStatus from './ControllerLiveStatus.vue';
import ControllerLiveFeed from './ControllerLiveFeed.vue';
import ControllerCompetitions from './ControllerCompetitions.vue';
import type { LiveFeedEvent } from '../model/live-feed.ts';
import type { DraftSaveTarget } from '../model/drafts.ts';

const control = useControlClient();
const model = createControllerModel(control);
const { status, profiles, rows, points, pointsError, pointsAllowed, error, busy, ready, commandSending } = model;
const { supported: draftsSupported, error: draftError, busy: draftBusy, loading: draftLoading } = model.draftStorage;
const { locale } = useI18n();
const labels = computed(() => locale.value === 'es' ? {
  title: 'Control remoto', controlling: 'Controlando', profiles: 'Perfiles guardados', connect: 'Conectar', disconnect: 'Desconectar', pair: 'Conectar a otra PC', origin: 'Dirección HTTPS del anfitrión', profile: 'Nombre del perfil', label: 'Nombre de este dispositivo', invite: 'Invitación o enlace privado', fingerprint: 'Huella SHA-256 aprobada (opcional)', verify: 'Comparé esta huella con el propietario', probe: 'Ver huella sin verificar', unverified: 'Huella sin verificar: compárala con el propietario antes de aprobarla.', submit: 'Solicitar acceso', pending: 'Esperando aprobación del propietario. Solicitud:', workflows: 'Workflows del anfitrión', empty: 'No hay workflows disponibles para este acceso.', draft: 'Borrador local', keep: 'El borrador se conserva al reconectar. No se envía al anfitrión.', conflict: 'El anfitrión cambió este workflow; tu borrador se conservó.', discard: 'Descartar borrador', readOnly: 'Guardar y activar definiciones todavía no está disponible. Los cambios permitidos requieren confirmación del anfitrión.', rights: 'Permisos efectivos', noProfiles: 'Crea un perfil para conectar con el anfitrión.', idle: 'Desconectado', pairing: 'Esperando aprobación', connecting: 'Conectando', ready: 'Conectado', reconnecting: 'Reconectando; recuperando estado', failed: 'No se pudo conectar', hidden: 'Los datos del anfitrión aparecerán al recuperar la conexión.',
} : {
  title: 'Remote controller', controlling: 'Controlling', profiles: 'Saved profiles', connect: 'Connect', disconnect: 'Disconnect', pair: 'Connect to another PC', origin: 'Host HTTPS address', profile: 'Profile name', label: 'This device name', invite: 'Private invitation or join link', fingerprint: 'Approved SHA-256 fingerprint (optional)', verify: 'I compared this fingerprint with the owner', probe: 'Inspect unverified fingerprint', unverified: 'Unverified fingerprint: compare it with the owner before approving it.', submit: 'Request access', pending: 'Waiting for owner approval. Request:', workflows: 'Host workflows', empty: 'No workflows are available for this access.', draft: 'Local draft', keep: 'Drafts survive reconnection. They are not sent to the host.', conflict: 'The host changed this workflow; your draft was retained.', discard: 'Discard draft', readOnly: 'Saving and enabling definitions are not available yet. Permitted changes require host confirmation.', rights: 'Effective permissions', noProfiles: 'Create a profile to connect to the host.', idle: 'Disconnected', pairing: 'Waiting for approval', connecting: 'Connecting', ready: 'Connected', reconnecting: 'Reconnecting; recovering state', failed: 'Connection failed', hidden: 'Host data will appear after the connection recovers.',
});
const phase = computed(() => labels.value[status.value.phase as 'idle' | 'pairing' | 'connecting' | 'ready' | 'reconnecting' | 'failed'] ?? status.value.phase);
const forgetLabels = computed(() => locale.value === 'es' ? {
  action: 'Olvidar perfil', title: '¿Olvidar este perfil?',
  message: 'Se eliminarán las credenciales, borradores e historial de comandos guardados en esta PC. Los comandos pendientes impiden eliminarlo. Las conexiones ya abiertas pueden seguir activas hasta que caduquen o el propietario revoque el acceso en el anfitrión.',
} : {
  action: 'Forget profile', title: 'Forget this profile?',
  message: 'The credentials, drafts and command history saved on this PC will be removed. Pending commands prevent removal. Existing connections may remain active until they expire or the owner revokes host access.',
});
const cmd = computed(() => locale.value === 'es' ? {
  disable:'Desactivar workflow',delete:'Eliminar workflow','disable-event':'Desactivar evento','delete-event':'Eliminar evento','adjust-points':'Ajustar puntos','create-action-draft':'Proponer acción nueva','update-action-draft':'Proponer reemplazo de acción','create-event-draft':'Proponer evento nuevo','update-event-draft':'Proponer reemplazo de evento',requested:'Cantidad solicitada',effective:'Ajuste aplicado',balance:'Saldo confirmado',disableTitle:'¿Desactivar este workflow en el anfitrión?',deleteTitle:'¿Eliminar este workflow del anfitrión?',
  disableMessage:'El workflow dejará de estar activo. El cambio se enviará una sola vez al anfitrión y requiere la revisión indicada.',deleteMessage:'Se eliminará el workflow del anfitrión. El borrador local se conserva. Esta operación requiere la revisión indicada.',
  journal:'Comandos de este perfil',waiting:'Procesando el comando en el anfitrión…',unknown:'El resultado no está confirmado. El comando puede haberse completado. No se reenvía automáticamente; las nuevas modificaciones están bloqueadas.',
  unavailable:'No se pudo guardar el registro del comando. Reabre el perfil y revisa el resultado antes de modificarlo.',check:'Consultar resultado',review:'Revisar resultado incierto',reviewTitle:'Revisar resultado incierto',
  reviewMessage:'Esto no deshace el cambio ni demuestra que haya fallado. Puede haberse completado en el anfitrión. El ID se conservará en el historial reciente como resultado incierto revisado. Cualquier cambio posterior requiere una nueva confirmación y revisión del estado actual.',
  reviewed:'Conservar como incierto',changed:'El estado cambió. Cierra esta confirmación y revisa de nuevo el estado seleccionado.',retainedDrafts:'Borradores locales conservados',hostUnavailableDraft:'El workflow del anfitrión no está disponible. Este borrador local no se puede enviar.',revision:'Revisión',history:'Historial reciente de comandos',pendingId:'ID del comando',
  outcome:{succeeded:'Confirmado por el anfitrión',rejected:'Rechazado sin aceptar el cambio','reviewed-unknown':'Resultado incierto revisado'},
} : {
  disable:'Disable workflow',delete:'Delete workflow','disable-event':'Disable event','delete-event':'Delete event','adjust-points':'Adjust points','create-action-draft':'Propose new action','update-action-draft':'Propose action replacement','create-event-draft':'Propose new event','update-event-draft':'Propose event replacement',requested:'Requested amount',effective:'Applied adjustment',balance:'Confirmed balance',disableTitle:'Disable this workflow on the host?',deleteTitle:'Delete this workflow from the host?',
  disableMessage:'The workflow will become inactive. The command is sent once to the host and requires the revision shown.',deleteMessage:'The workflow will be removed from the host. Your local draft is retained. This command requires the revision shown.',
  journal:'Commands for this profile',waiting:'Processing the host command…',unknown:'The result is unconfirmed. The command may have completed. It is never resent automatically; new changes are blocked.',
  unavailable:'The command journal could not be saved. Reopen the profile and review its outcome before changing host state.',check:'Check outcome',review:'Review unknown outcome',reviewTitle:'Review unknown outcome',
  reviewMessage:'This does not undo the change or prove it failed. It may have completed on the host. Its ID stays in recent history as a reviewed unknown outcome. Any later change requires a new confirmation against the current host state.',
  reviewed:'Keep outcome unknown',changed:'State changed. Close this confirmation and review the selected state again.',retainedDrafts:'Retained local drafts',hostUnavailableDraft:'The host workflow is unavailable. This local draft cannot be sent.',revision:'Revision',history:'Recent command history',pendingId:'Command ID',
  outcome:{succeeded:'Confirmed by host',rejected:'Rejected before accepting the change','reviewed-unknown':'Reviewed unknown outcome'},
});
const pts = computed(() => locale.value === 'es' ? {
  title:'Puntos del anfitrión', search:'Buscar visor por ID', find:'Consultar visor', top:'Primeros puestos permitidos', empty:'No hay visores en esta vista.', loading:'Actualizando puntos…',
  amount:'Cantidad a sumar o restar', adjust:'Ajustar puntos', confirm:'¿Aplicar este ajuste en el anfitrión?', note:'La cantidad se aplica al saldo que exista al aceptar el comando. Puede aplicarse el bono de suscriptor; las penalizaciones no bajan de cero.',
  create:'No existe un registro de puntos para este ID. El ajuste creará uno.', result:'Saldo actual', selected:'Visor seleccionado', clear:'Volver a los primeros puestos',
} : {
  title:'Host points', search:'Find viewer by ID', find:'Look up viewer', top:'Top permitted viewers', empty:'No viewers in this view.', loading:'Updating points…',
  amount:'Amount to add or subtract', adjust:'Adjust points', confirm:'Apply this adjustment on the host?', note:'The amount applies to the balance at acceptance. Subscriber bonuses may apply; penalties cannot reduce the balance below zero.',
  create:'This ID has no points record. The adjustment will create one.', result:'Current balance', selected:'Selected viewer', clear:'Back to top viewers',
});
const pointSearch = ref(''); const pointAmount = ref<string | number>('');
const pointsSection = ref<HTMLElement | null>(null);
function canLookupFeedViewer(event: LiveFeedEvent): boolean {
  const id = event.authorLookupId;
  const viewers = status.value.hello?.resourceRestrictions?.['viewer'];
  return !!id && ready.value && pointsAllowed.value && !busy.value && model.liveFeed.allowed.value &&
    (!viewers || viewers.includes(id)) && model.liveFeed.events.value.some((item) => item.cursor === event.cursor && item.authorLookupId === id);
}
function lookupFeedViewer(event: LiveFeedEvent): void {
  const id = event.authorLookupId;
  if (!id || !canLookupFeedViewer(event)) return;
  pointSearch.value = id; pointAmount.value = '';
  void model.selectPoints(id);
  void nextTick(() => { pointsSection.value?.focus(); pointsSection.value?.scrollIntoView?.({ block: 'start' }); });
}
watch(() => points.value?.selectedId ?? null, () => { pointAmount.value = ''; });
const dft = computed(() => locale.value === 'es' ? {
  limit:'Máximo guardado: 256 KiB de texto por borrador.',
  save:'Guardar borrador en esta PC',title:'¿Guardar este texto en esta PC?',consent:'Se guardará el texto completo, incluidos los valores sensibles que contenga, en el perfil privado de este controlador. No se envía al anfitrión. Confirma solo si deseas conservarlo al cerrar la app.',
  reload:'Revisar copia guardada',saved:'Copia guardada en esta PC',unsaved:'Hay cambios locales sin guardar.',matches:'Este texto coincide con la copia guardada en esta PC.',remove:'Eliminar copia guardada; conservar texto',
  compare:'Versión actual del anfitrión',base:'Revisión base del borrador',rebase:'Usar la revisión actual para esta combinación local',merge:'Compara ambos textos y edita tu borrador antes de elegir la nueva revisión. Esto no guarda ni modifica el anfitrión.',
} : {
  limit:'Saved drafts can contain up to 256 KiB of text each.',
  save:'Save draft on this PC',title:'Save this text on this PC?',consent:'The complete text, including any sensitive values it contains, will be saved in this controller’s private profile. It is not sent to the host. Confirm only if you want to keep it after closing the app.',
  reload:'Review saved copy',saved:'Saved copy on this PC',unsaved:'There are unsaved local changes.',matches:'This text matches the saved copy on this PC.',remove:'Delete saved copy; keep text',
  compare:'Current host version',base:'Draft base revision',rebase:'Use the current revision for this local merge',merge:'Compare both texts and edit your draft before choosing the new revision. This does not save or change the host.',
});
const draftErrorText = computed(() => {
  const code=draftError.value;if(!code)return '';
  if(locale.value==='es') {
    if(['draft_conflict','draft_review_required','transport','timeout'].includes(code))return 'El guardado o borrado no está confirmado, o cambió la copia local. Revisa la copia guardada antes de reemplazarla.';
    if(code==='draft_limit')return 'Se alcanzó el límite de borradores guardados. Tu texto y las copias existentes se conservaron.';
    if(['stale_view','not_ready','forbidden'].includes(code))return 'Cambió la conexión o el acceso. Recupera la vista autorizada antes de guardar.';
    return 'No se pudo acceder a los borradores guardados. Conserva el texto sin guardar y revisa de nuevo la copia local.';
  }
  if(['draft_conflict','draft_review_required','transport','timeout'].includes(code))return 'The save or removal is unconfirmed, or the local copy changed. Review the saved copy before replacing it.';
  if(code==='draft_limit')return 'The saved draft limit was reached. Your text and existing copies were preserved.';
  if(['stale_view','not_ready','forbidden'].includes(code))return 'The connection or access changed. Recover the authorized view before saving.';
  return 'Saved drafts are unavailable. Keep your unsaved text and review the local copy again.';
});
const pointId = computed(() => points.value?.selectedId ?? '');
function requestPoints(): void {
  if (!String(pointAmount.value).trim()) return;
  const target = model.preparePoints(pointId.value, Number(pointAmount.value));
  if (target) confirmation.value = { kind: 'points', target };
}
type Confirmation = { kind:'draft';target:DraftSaveTarget;origin:string;hostId:string } | { kind:'points';target:PointsTarget } | { kind:'mutation';target:CommandTarget } | { kind:'review';target:ReviewTarget;intent:PendingCommand;origin:string;hostId:string } | { kind:'forget';target:ProfileForgetTarget };
const confirmation = ref<Confirmation | null>(null);
const confirmOpen = computed({get:()=>confirmation.value!==null,set:(open:boolean)=>{if(!open)confirmation.value=null;}});
const confirmationCurrent = computed(()=>!!confirmation.value && (confirmation.value.kind==='draft' ? model.draftStorage.current(confirmation.value.target) : confirmation.value.kind==='points' ? model.pointsIsCurrent(confirmation.value.target) : confirmation.value.kind==='mutation' ? model.commandIsCurrent(confirmation.value.target) : confirmation.value.kind==='forget' ? model.profileForgetCurrent(confirmation.value.target) : model.reviewIsCurrent(confirmation.value.target)));
function requestForget(profile: RemoteProfile): void {
  const target = model.prepareForget(profile);
  if (target) confirmation.value = { kind: 'forget', target };
}
function requestSaveDraft(): void {
  const target=model.draftStorage.prepare(selected.value);
  if(target && status.value.hello && status.value.origin)confirmation.value={kind:'draft',target,origin:status.value.origin,hostId:status.value.hello.hostId};
}
function requestChange(operation:WorkflowOperation): void {
  if(!row.value)return;const target=model.prepareCommand(row.value,operation);if(target)confirmation.value={kind:'mutation',target};
}
function requestReview(): void {
  const target=model.prepareReview();const journal=status.value.commandJournal;
  if(target&&journal?.pending)confirmation.value={kind:'review',target,intent:{...journal.pending},origin:journal.binding.origin,hostId:journal.binding.hostId};
}
async function confirmCommand(): Promise<void> {
  const item=confirmation.value;if(!item||!confirmationCurrent.value)return;confirmation.value=null;
  if(item.kind==='draft')await model.draftStorage.save(item.target);else if(item.kind==='points') {if(await model.adjustPoints(item.target))pointAmount.value='';}else if(item.kind==='mutation')await model.changeWorkflow(item.target);else if(item.kind==='forget')await model.forgetProfile(item.target);else await model.reviewUnknown(item.target);
}
watch(() => JSON.stringify([status.value.epoch, status.value.origin, status.value.hello?.connectionId,
  status.value.hello?.hostId, status.value.hello?.hostInstanceId, status.value.hello?.principalId,
  status.value.hello?.sessionId, status.value.hello?.grantRevision, status.value.hello?.capabilities,
  status.value.hello?.resourceRestrictions]), () => { confirmation.value=null; pointSearch.value=''; pointAmount.value=''; });
const selected = ref('');
const row = computed(() => rows.value.find((r) => r.version.id === selected.value));
const retainedDrafts = computed(() => model.draftIds().filter((id) => !rows.value.some((r) => r.version.id === id)));
const draftText = computed(() => row.value ? model.draft(row.value.version.id)?.text ?? JSON.stringify(row.value.graph, null, 2) : retainedDrafts.value.includes(selected.value) ? model.draft(selected.value)?.text ?? '' : '');
const conflict = computed(() => row.value && model.draft(row.value.version.id) && model.draft(row.value.version.id)?.revision !== row.value.version.revision);
const savedDraft = computed(() => model.draftStorage.saved(selected.value));
const draftMatches = computed(() => !!savedDraft.value && model.draft(selected.value)?.text === savedDraft.value.text && model.draft(selected.value)?.revision === savedDraft.value.baseRevision);
watch([selected,ready],()=>{if(selected.value && ready.value)void model.draftStorage.load(selected.value);});
const form = ref({ profile: '', origin: '', label: 'Moderator', invitation: '', fingerprint: '', confirmPin: false });
const fingerprint = ref('');
const pending = ref('');
const pairing = ref(false);
let stopped = false;
let pollTimer: ReturnType<typeof setTimeout> | undefined;
async function probe(): Promise<void> {
  try { fingerprint.value = (await control.call<{ fingerprint: string }>('controller.probe', { origin: form.value.origin })).fingerprint; }
  catch (failure) { error.value = failure instanceof Error ? failure.message : 'Trust check failed'; }
}
async function poll(): Promise<void> {
  try {
    const result = await control.call<{ approved: boolean; profile?: string }>('controller.pair.poll');
    if (stopped) return;
    if (result.approved && result.profile) { pairing.value = false; pending.value = ''; await model.inventory(); await model.connect(result.profile); }
    else pollTimer = setTimeout(() => { void poll(); }, 1500);
  } catch (failure) { pairing.value = false; error.value = failure instanceof Error ? failure.message : 'Pairing failed'; }
}
async function pair(): Promise<void> {
  pairing.value = true; error.value = '';
  const params = { ...form.value, fingerprint: form.value.fingerprint.trim() || null };
  form.value.invitation = '';
  try {
    const reply = await control.call<{ requestId: string }>('controller.pair', params);
    if (stopped) return;
    pending.value = reply.requestId; await poll();
  } catch (failure) { pairing.value = false; error.value = failure instanceof Error ? failure.message : 'Pairing failed'; }
}
async function disconnect(): Promise<void> { if (pollTimer) clearTimeout(pollTimer); pending.value = ''; pairing.value = false; await model.disconnect(); }
onMounted(() => { void model.start(window.__tiktools_controller__?.profile).then(() => {
  if (status.value.phase === 'pairing' && !stopped) {
    pending.value = status.value.pairingRequestId ?? ''; pairing.value = true; void poll();
  }
}); });
onUnmounted(() => { stopped = true; if (pollTimer) clearTimeout(pollTimer); model.stop(); });
</script>

<template>
  <main class="controller">
    <header
      class="controller__banner"
      aria-live="polite"
    >
      <div><h1>{{ status.origin ? `${labels.controlling} ${status.origin}` : labels.title }}</h1><p>{{ phase }} <span v-if="status.errorCode">· {{ status.errorCode }}</span></p><small v-if="status.hello">Host {{ status.hello.hostId }}</small></div>
      <button
        v-if="status.phase !== 'idle'"
        :disabled="busy"
        @click="disconnect"
      >
        {{ labels.disconnect }}
      </button>
    </header>
    <p
      v-if="error"
      role="alert"
      class="controller__error"
    >
      {{ error }}
    </p>
    <section
      v-if="status.commandJournal && (status.commandBusy || status.commandJournal.pending || status.commandJournal.unavailable || status.commandJournal.recent.length)"
      class="controller__panel controller__journal"
      :aria-label="cmd.journal"
    >
      <h2>{{ cmd.journal }}</h2>
      <code>{{ status.commandJournal.binding.origin }}</code>
      <code>Host {{ status.commandJournal.binding.hostId }}</code>
      <p
        v-if="status.commandJournal.unavailable"
        role="alert"
      >
        {{ cmd.unavailable }}
      </p>
      <p
        v-if="status.commandBusy"
        role="status"
      >
        {{ cmd.waiting }}
      </p>
      <template v-if="status.commandJournal.pending">
        <p v-if="!status.commandBusy">
          {{ cmd.unknown }}
        </p>
        <p>{{ cmd[status.commandJournal.pending.operation] }} · {{ status.commandJournal.pending.resourceId }}</p>
        <p v-if="status.commandJournal.pending.delta !== undefined">
          {{ cmd.requested }}: {{ status.commandJournal.pending.delta }} {{ status.commandJournal.pending.currencyName }}
        </p>
        <p><span v-if="status.commandJournal.pending.expectedRevision">{{ cmd.revision }} {{ status.commandJournal.pending.expectedRevision }} · </span>{{ new Date(status.commandJournal.pending.createdAt).toLocaleString() }}</p>
        <p>{{ cmd.pendingId }}:</p><code>{{ status.commandJournal.pending.commandId }}</code>
        <p v-if="status.commandErrorCode">
          {{ status.commandErrorCode }}
        </p>
        <div class="controller__actions">
          <button
            :disabled="!model.canResolve.value"
            @click="model.resolveCommand()"
          >
            {{ cmd.check }}
          </button>
          <button
            :disabled="!model.canReview.value"
            @click="requestReview"
          >
            {{ cmd.review }}
          </button>
        </div>
      </template>
      <template v-if="status.commandJournal.recent[0]">
        <p aria-live="polite">
          {{ cmd.outcome[status.commandJournal.recent[0].outcome] }} · {{ status.commandJournal.recent[0].intent.resourceId }}
        </p>
        <details>
          <summary>{{ cmd.history }}</summary>
          <article
            v-for="record in status.commandJournal.recent"
            :key="record.intent.commandId"
          >
            <p>{{ cmd[record.intent.operation] }} · {{ record.intent.resourceId }} · {{ cmd.outcome[record.outcome] }}</p>
            <code>{{ record.intent.commandId }}</code>
            <p v-if="record.intent.delta !== undefined">
              {{ cmd.requested }}: {{ record.intent.delta }} {{ record.intent.currencyName }}
            </p>
            <p v-if="record.pointsResult">
              {{ cmd.effective }}: {{ record.pointsResult.delta }} {{ record.intent.currencyName }} · {{ cmd.balance }}: {{ record.pointsResult.totalPoints }} {{ record.intent.currencyName }}
            </p>
            <p v-if="record.revision">
              {{ cmd.revision }} {{ record.revision }}
            </p>
            <p v-if="record.errorCode">
              {{ record.errorCode }}
            </p>
          </article>
        </details>
      </template>
    </section>
    <section class="controller__panel">
      <h2>{{ labels.profiles }}</h2><p v-if="!profiles.length">
        {{ labels.noProfiles }}
      </p>
      <div
        v-for="profile in profiles"
        :key="profile.name"
        class="controller__profile"
      >
        <div><strong>{{ profile.name }}</strong><p>{{ profile.origin }}</p></div><button
          :disabled="busy || pairing"
          @click="model.connect(profile.name)"
        >
          {{ labels.connect }}
        </button>
        <button
          :disabled="!model.prepareForget(profile) || pairing"
          @click="requestForget(profile)"
        >
          {{ forgetLabels.action }}
        </button>
      </div>
    </section>
    <details
      v-if="['idle', 'failed'].includes(status.phase)"
      class="controller__panel"
    >
      <summary>{{ labels.pair }}</summary>
      <form
        autocomplete="off"
        @submit.prevent="pair"
      >
        <label>{{ labels.origin }}<input
          v-model="form.origin"
          type="url"
          placeholder="https://host.example"
          required
          :disabled="pairing"
        ></label>
        <label>{{ labels.profile }}<input
          v-model="form.profile"
          pattern="[A-Za-z0-9_][A-Za-z0-9_-]{0,63}"
          maxlength="64"
          required
          :disabled="pairing"
        ></label>
        <label>{{ labels.label }}<input
          v-model="form.label"
          maxlength="128"
          required
          :disabled="pairing"
        ></label>
        <label>{{ labels.invite }}<input
          v-model="form.invitation"
          type="password"
          autocomplete="new-password"
          maxlength="8192"
          required
          :disabled="pairing"
        ></label>
        <button
          type="button"
          :disabled="pairing || !form.origin"
          @click="probe"
        >
          {{ labels.probe }}
        </button>
        <p v-if="fingerprint">
          {{ labels.unverified }}<code>{{ fingerprint }}</code>
        </p>
        <label>{{ labels.fingerprint }}<input
          v-model="form.fingerprint"
          maxlength="100"
          :disabled="pairing"
        ></label>
        <label
          v-if="form.fingerprint"
          class="controller__checkbox"
        ><input
          v-model="form.confirmPin"
          type="checkbox"
          :disabled="pairing"
        >{{ labels.verify }}</label>
        <button
          type="submit"
          :disabled="pairing || !!form.fingerprint && !form.confirmPin"
        >
          {{ labels.submit }}
        </button>
      </form>
    </details>
    <p
      v-if="pending"
      role="status"
    >
      {{ labels.pending }} <code>{{ pending }}</code>
    </p>
    <ControllerLiveStatus :model="model.live" />
    <ControllerLiveFeed :model="model.liveFeed" :can-lookup-points="canLookupFeedViewer" :show-ambiguous-viewer="ready && pointsAllowed" @lookup-points="lookupFeedViewer" />
    <ControllerCompetitions :model="model.competitions" />
    <ControllerAutomations
      :model="model.automation"
      :epoch="status.epoch"
      :busy="busy || commandSending"
    />
    <ControllerProposals :model="model.proposals" :automation="model.automation" />
    <section
      v-if="ready && pointsAllowed"
      ref="pointsSection"
      class="controller__panel controller__points"
      :aria-label="pts.title"
      tabindex="-1"
    >
      <h2>{{ pts.title }}</h2>
      <p
        v-if="pointsError"
        role="alert"
      >
        {{ pointsError }}
      </p>
      <form
        class="controller__pointSearch"
        @submit.prevent="model.selectPoints(pointSearch)"
      >
        <label>{{ pts.search }}<input
          v-model="pointSearch"
          maxlength="128"
          autocomplete="off"
          spellcheck="false"
          :disabled="busy"
        ></label>
        <button
          type="submit"
          :disabled="busy || !pointSearch.trim()"
        >
          {{ pts.find }}
        </button>
        <button
          v-if="points?.selectedId"
          type="button"
          @click="pointSearch='';model.selectPoints('')"
        >
          {{ pts.clear }}
        </button>
      </form>
      <p
        v-if="!points || points.loading"
        role="status"
      >
        {{ pts.loading }}
      </p>
      <template v-if="points && !points.loading && !points.errorCode">
        <h3 v-if="!points.selectedId">
          {{ pts.top }} ({{ points.rows?.length ?? 0 }})
        </h3>
        <p v-if="!points.selectedId && !points.rows?.length">
          {{ pts.empty }}
        </p>
        <ul class="controller__pointRows">
          <li
            v-for="item in points.rows"
            :key="item.version.id"
          >
            <button
              :aria-pressed="points.selectedId===item.version.id"
              @click="pointSearch=item.version.id;model.selectPoints(item.version.id)"
            >
              {{ item.version.id }} · {{ item.viewer.points }} {{ points.currencyName }}
            </button>
          </li>
        </ul>
        <div
          v-if="points.selectedId"
          class="controller__pointTarget"
        >
          <h3>{{ pts.selected }}: {{ points.selectedId }}</h3>
          <p v-if="points.selected">
            {{ pts.result }}: {{ points.selected.viewer.points }} {{ points.currencyName }}
          </p>
          <p v-else-if="points.selectedVersion?.revision==='0'">
            {{ pts.create }}
          </p>
          <form
            v-if="status.hello?.capabilities.includes('points.adjust') && status.hello.features['pointsAdjust']"
            @submit.prevent="requestPoints"
          >
            <label>{{ pts.amount }} ({{ points.currencyName }})<input
              v-model="pointAmount"
              type="number"
              step="0.01"
              min="-1000000000"
              max="1000000000"
              :disabled="!model.pointsCanAdjust()"
            ></label>
            <button
              type="submit"
              :disabled="!model.preparePoints(pointId, Number(pointAmount)) || !String(pointAmount).trim()"
            >
              {{ pts.adjust }}
            </button>
          </form>
        </div>
      </template>
    </section>
    <section
      v-if="ready && status.hello?.capabilities.includes('workflows.read')"
      class="controller__panel"
    >
      <h2>{{ labels.workflows }}</h2><p>{{ labels.readOnly }}</p><p v-if="!rows.length">
        {{ labels.empty }}
      </p>
      <div class="controller__workflows">
        <nav :aria-label="labels.workflows">
          <button
            v-for="item in rows"
            :key="item.version.id"
            :aria-pressed="selected === item.version.id"
            @click="selected = item.version.id"
          >
            {{ item.graph['name'] || item.version.id }} · {{ item.version.revision }}
          </button>
          <template v-if="retainedDrafts.length">
            <h3>{{ cmd.retainedDrafts }}</h3>
            <button
              v-for="id in retainedDrafts"
              :key="`draft-${id}`"
              :aria-pressed="selected === id"
              @click="selected = id"
            >
              {{ id }} · {{ labels.draft }}
            </button>
          </template>
        </nav>
        <div v-if="row || retainedDrafts.includes(selected)">
          <div
            v-if="row"
            class="controller__actions"
          >
            <button
              v-if="status.hello?.capabilities.includes('workflows.toggle') && status.hello.features['workflowDisable'] && row.graph['enabled'] === true"
              :disabled="!model.canChange('disable')"
              @click="requestChange('disable')"
            >
              {{ cmd.disable }}
            </button>
            <button
              v-if="status.hello?.capabilities.includes('workflows.edit') && status.hello.features['workflowDelete']"
              :disabled="!model.canChange('delete')"
              @click="requestChange('delete')"
            >
              {{ cmd.delete }}
            </button>
          </div>
          <h3>{{ labels.draft }}</h3><p>{{ row ? labels.keep : cmd.hostUnavailableDraft }}</p><p
            v-if="conflict"
            role="status"
          >
            {{ labels.conflict }}
          </p>
          <section
            v-if="conflict && row"
            class="controller__comparison"
          >
            <h4>{{ dft.compare }} · {{ row.version.revision }}</h4>
            <textarea
              :value="JSON.stringify(row.graph, null, 2)"
              :aria-label="dft.compare"
              readonly
              spellcheck="false"
            />
            <p>{{ dft.base }}: {{ model.draft(selected)?.revision }}</p>
            <p>{{ dft.merge }}</p>
            <button
              :disabled="draftBusy || draftLoading"
              @click="model.rebaseDraft(row)"
            >
              {{ dft.rebase }} · {{ row.version.revision }}
            </button>
          </section>
          <textarea
            :value="draftText"
            :aria-label="labels.draft"
            spellcheck="false"
            :disabled="draftBusy || draftLoading"
            @input="row ? model.edit(row, ($event.target as HTMLTextAreaElement).value) : model.editRetainedDraft(selected, ($event.target as HTMLTextAreaElement).value)"
          /><button
            v-if="model.draft(selected)"
            :disabled="draftBusy || draftLoading"
            @click="model.discard(selected)"
          >
            {{ labels.discard }}
          </button>
          <section
            v-if="draftsSupported"
            class="controller__draftStorage"
          >
            <p
              v-if="draftError"
              role="alert"
            >
              {{ draftErrorText }}
            </p>
            <p>{{ dft.limit }}</p>
            <button
              :disabled="!model.draftStorage.prepare(selected)"
              @click="requestSaveDraft"
            >
              {{ dft.save }}
            </button>
            <button
              :disabled="draftBusy || draftLoading"
              @click="model.draftStorage.load(selected, true)"
            >
              {{ dft.reload }}
            </button>
            <p v-if="model.draft(selected)">
              {{ draftMatches ? dft.matches : dft.unsaved }}
            </p>
            <details v-if="savedDraft">
              <summary>{{ dft.saved }} · {{ savedDraft.baseRevision }}</summary>
              <textarea
                :value="savedDraft.text"
                :aria-label="dft.saved"
                readonly
                spellcheck="false"
              />
              <button
                :disabled="draftBusy || draftLoading"
                @click="model.draftStorage.remove(selected, false)"
              >
                {{ dft.remove }}
              </button>
            </details>
          </section>
        </div>
      </div>
      <details>
        <summary>{{ labels.rights }}</summary><ul>
          <li
            v-for="capability in status.hello?.capabilities"
            :key="capability"
          >
            {{ capability }}
          </li>
        </ul>
      </details>
    </section>
    <p v-else-if="!ready && !['idle', 'pairing', 'failed'].includes(status.phase)">
      {{ labels.hidden }}
    </p>
    <BaseConfirmDialog
      v-model:open="confirmOpen"
      :title="confirmation?.kind === 'draft' ? dft.title : confirmation?.kind === 'forget' ? forgetLabels.title : confirmation?.kind === 'points' ? pts.confirm : confirmation?.kind === 'review' ? cmd.reviewTitle : confirmation?.kind === 'mutation' && confirmation.target.operation === 'delete' ? cmd.deleteTitle : cmd.disableTitle"
      :message="confirmation?.kind === 'draft' ? dft.consent : confirmation?.kind === 'forget' ? forgetLabels.message : confirmation?.kind === 'points' ? pts.note : confirmation?.kind === 'review' ? cmd.reviewMessage : confirmation?.kind === 'mutation' && confirmation.target.operation === 'delete' ? cmd.deleteMessage : cmd.disableMessage"
      :confirm-label="confirmation?.kind === 'draft' ? dft.save : confirmation?.kind === 'forget' ? forgetLabels.action : confirmation?.kind === 'points' ? pts.adjust : confirmation?.kind === 'review' ? cmd.reviewed : confirmation?.kind === 'mutation' && confirmation.target.operation === 'delete' ? cmd.delete : cmd.disable"
      :busy="commandSending || busy || draftBusy || !confirmationCurrent"
      :danger="confirmation?.kind !== 'draft'"
      @confirm="confirmCommand"
    >
      <template v-if="confirmation?.kind === 'forget'">
        <p>{{ confirmation.target.name }}</p><code>{{ confirmation.target.origin }}</code><code>Host {{ confirmation.target.hostId }}</code>
      </template>
      <template v-else-if="confirmation?.kind === 'draft'">
        <code>{{ confirmation.origin }}</code><code>Host {{ confirmation.hostId }}</code>
        <p>{{ confirmation.target.id }} · {{ dft.base }} {{ confirmation.target.revision }}</p>
      </template>
      <template v-else-if="confirmation?.kind === 'points'">
        <code>{{ confirmation.target.origin }}</code><code>Host {{ confirmation.target.hostId }}</code>
        <p>{{ confirmation.target.uniqueId }} · {{ confirmation.target.delta }} {{ confirmation.target.currencyName }}</p>
        <p v-if="confirmation.target.creates">
          {{ pts.create }}
        </p>
      </template>
      <template v-else-if="confirmation?.kind === 'mutation'">
        <code>{{ confirmation.target.origin }}</code><code>Host {{ confirmation.target.hostId }}</code>
        <p>{{ confirmation.target.id }} · {{ cmd.revision }} {{ confirmation.target.expectedRevision }}</p>
      </template>
      <template v-else-if="confirmation?.kind === 'review'">
        <code>{{ confirmation.origin }}</code><code>Host {{ confirmation.hostId }}</code>
        <p>{{ confirmation.intent.resourceId }}</p>
        <p v-if="confirmation.intent.delta !== undefined">
          {{ cmd.requested }}: {{ confirmation.intent.delta }} {{ confirmation.intent.currencyName }}
        </p>
        <code>{{ confirmation.intent.commandId }}</code>
      </template>
      <p v-if="confirmation && !confirmationCurrent">
        {{ cmd.changed }}
      </p>
    </BaseConfirmDialog>
  </main>
</template>

<style scoped>
.controller { max-width: 1100px; height: 100%; overflow-y: auto; margin: auto; padding: 24px; color: var(--color-text); }
.controller__banner, .controller__profile { display:flex; justify-content:space-between; gap:20px; align-items:center; }
.controller__banner { position:sticky; top:0; z-index:1; padding:16px; background:var(--color-bg-panel); border-bottom:3px solid #28bec4; }
.controller__banner > div { min-width:0; flex:1; }.controller__banner h1 { overflow-wrap:anywhere; }
.controller h1 { font-size:22px; margin:0; }.controller h2 { font-size:18px; }.controller p { margin:8px 0; }
.controller__panel { padding:20px; margin-top:18px; border:1px solid var(--color-border-subtle); border-radius:8px; background:var(--color-bg-panel); }
.controller__profile { padding:12px 0; border-bottom:1px solid var(--color-border-subtle); }
.controller button { cursor:pointer; padding:8px 14px; border:1px solid var(--color-border-subtle); border-radius:6px; color:inherit; background:var(--color-bg-panel); }
.controller button:disabled { opacity:.5; cursor:default; }.controller button[aria-pressed=true] { border-color:#28bec4; }
.controller label { display:grid; gap:6px; margin:14px 0; }.controller input, .controller textarea { width:100%; padding:10px; border:1px solid var(--color-border-subtle); border-radius:5px; color:inherit; background:var(--color-bg-field); }
.controller .controller__checkbox { display:flex; align-items:center; }.controller__checkbox input { width:auto; }
.controller code { display:block; overflow-wrap:anywhere; }.controller__error { color:#f5848b; }.controller summary { cursor:pointer; }
.controller__workflows { display:grid; grid-template-columns:220px minmax(0,1fr); gap:20px; margin:20px 0; }.controller nav { display:flex; flex-direction:column; gap:8px; }.controller textarea { min-height:330px; font-family:monospace; }
.controller__journal p, .controller__points, dialog p { overflow-wrap:anywhere; }
.controller__pointRows { list-style:none; padding:0; display:grid; gap:6px; }.controller__pointRows button { width:100%; text-align:left; overflow-wrap:anywhere; }.controller__pointTarget { border-top:1px solid var(--color-border-subtle); margin-top:16px; padding-top:12px; }
.controller__actions { display:flex; flex-wrap:wrap; gap:10px; margin:12px 0; }.controller__journal article { border-top:1px solid var(--color-border-subtle); margin-top:12px; padding-top:8px; }
@media(max-width:700px) { .controller { padding:10px; }.controller__banner { position:static; }.controller__workflows { grid-template-columns:1fr; } }
</style>
