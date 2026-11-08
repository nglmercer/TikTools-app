<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AutomationModel, AutomationRow, AutomationTarget, EventOperation } from '../model/automations.ts';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
const props = defineProps<{ model: AutomationModel; epoch: string; busy: boolean }>();
const { locale } = useI18n();
const text = computed(() => locale.value === 'es' ? {
  title: 'Automatizaciones del anfitrión', note: 'Se muestra el estado configurado. Las definiciones pendientes de revisión no se ejecutan, aunque estén marcadas activas.',
  event: 'Evento', action: 'Acción', enabled: 'Activo', disabled: 'Inactivo', empty: 'No hay automatizaciones disponibles para este acceso.', loading: 'Recuperando estado de automatizaciones…',
  disable: 'Desactivar evento', delete: 'Eliminar evento', disableTitle: '¿Desactivar este evento en el anfitrión?', deleteTitle: '¿Eliminar este evento del anfitrión?',
  disableMessage: 'El evento completo dejará de estar activo. El anfitrión confirmará el cambio para la revisión indicada. Las acciones ya iniciadas pueden terminar.',
  deleteMessage: 'Se eliminará el evento completo del anfitrión para la revisión indicada. Las acciones ya iniciadas pueden terminar.',
  revision: 'Revisión', changed: 'El estado o el acceso cambió. Cierra esta confirmación y revisa el evento de nuevo.',
} : {
  title: 'Host automations', note: 'Configured status is shown. Definitions awaiting review do not run, even if marked enabled.',
  event: 'Event', action: 'Action', enabled: 'Enabled', disabled: 'Disabled', empty: 'No automations are available for this access.', loading: 'Recovering automation status…',
  disable: 'Disable event', delete: 'Delete event', disableTitle: 'Disable this event on the host?', deleteTitle: 'Delete this event from the host?',
  disableMessage: 'The complete event will become inactive. The host must confirm the change for the revision shown. Already started actions may finish.',
  deleteMessage: 'The complete event will be removed from the host for the revision shown. Already started actions may finish.',
  revision: 'Revision', changed: 'The state or access changed. Close this confirmation and review the event again.',
});
const target = ref<AutomationTarget | null>(null);
const open = computed({ get: () => target.value !== null, set: (value: boolean) => { if (!value) target.value = null; } });
const current = computed(() => target.value !== null && props.model.current(target.value));
function request(item: AutomationRow, operation: EventOperation): void { target.value = props.model.prepare(item, operation); }
async function confirm(): Promise<void> { const submitted = target.value; if (!submitted || !current.value) return; target.value = null; await props.model.change(submitted); }
watch([() => props.epoch, () => props.model.identity.value, () => props.model.allowed.value], () => { target.value = null; });
</script>

<template>
  <section
    v-if="model.allowed.value"
    class="automation-board"
    :aria-label="text.title"
  >
    <h2>{{ text.title }}</h2>
    <p>{{ text.note }}</p>
    <p
      v-if="model.error.value"
      role="alert"
    >
      {{ model.error.value }}
    </p>
    <p
      v-else-if="!model.ready.value"
      role="status"
    >
      {{ text.loading }}
    </p>
    <template v-if="model.ready.value">
      <p v-if="!model.rows.value.length">
        {{ text.empty }}
      </p>
      <ul>
        <li
          v-for="item in model.rows.value"
          :key="`${item.summary.kind}:${item.summary.id}`"
        >
          <div>
            <strong>{{ item.summary.name || item.summary.id }}</strong>
            <p>{{ item.summary.kind === 'event' ? text.event : text.action }} · {{ item.summary.id }}</p>
            <p>{{ item.summary.enabled ? text.enabled : text.disabled }} · {{ text.revision }} {{ item.version.revision }}</p>
          </div>
          <div
            v-if="item.summary.kind === 'event'"
            class="automation-board__actions"
          >
            <button
              :disabled="!model.canChange(item, 'disable-event')"
              @click="request(item, 'disable-event')"
            >
              {{ text.disable }}
            </button>
            <button
              :disabled="!model.canChange(item, 'delete-event')"
              @click="request(item, 'delete-event')"
            >
              {{ text.delete }}
            </button>
          </div>
        </li>
      </ul>
    </template>
  </section>
  <BaseConfirmDialog
    v-model:open="open"
    :title="target?.operation === 'delete-event' ? text.deleteTitle : text.disableTitle"
    :message="target?.operation === 'delete-event' ? text.deleteMessage : text.disableMessage"
    :confirm-label="target?.operation === 'delete-event' ? text.delete : text.disable"
    :busy="busy || !current"
    danger
    @confirm="confirm"
  >
    <template v-if="target">
      <code>{{ target.origin }}</code><code>Host {{ target.hostId }}</code>
      <p>{{ target.name }} · {{ text.event }} {{ target.id }}</p>
      <p>{{ text.revision }} {{ target.expectedRevision }}</p>
      <p
        v-if="!current"
        role="status"
      >
        {{ text.changed }}
      </p>
    </template>
  </BaseConfirmDialog>
</template>

<style scoped>
.automation-board { padding:20px; margin-top:18px; border:1px solid var(--color-border-subtle); border-radius:8px; background:var(--color-bg-panel); overflow-wrap:anywhere; }
h2 { font-size:18px; } p { margin:8px 0; } ul { list-style:none; padding:0; margin:0; }
li { display:flex; gap:16px; align-items:center; justify-content:space-between; padding:14px 0; border-top:1px solid var(--color-border-subtle); }
li > div { min-width:0; }.automation-board__actions { display:flex; flex-wrap:wrap; gap:8px; flex-shrink:0; }
button { cursor:pointer; padding:8px 14px; border:1px solid var(--color-border-subtle); border-radius:6px; color:inherit; background:var(--color-bg-panel); }
button:disabled { opacity:.5; cursor:default; } code { display:block; overflow-wrap:anywhere; }
@media(max-width:700px) { li { align-items:stretch; flex-direction:column; }.automation-board__actions { flex-shrink:1; } }
</style>
