<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { MessageSchema } from '@/shared/i18n/i18n.ts';
import type { ProposalModel, ProposalConfirmation } from '../model/proposals.ts';
import type { AutomationModel } from '../model/automations.ts';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import ControllerProposalBuilder from './ControllerProposalBuilder.vue';
const props = defineProps<{ model: ProposalModel; automation?: AutomationModel }>();
const { tm } = useI18n();
const family = ref<'action' | 'event'>('action'); const targetId = ref('');
const dialog = ref<ProposalConfirmation | null>(null);
const open = computed({ get: () => dialog.value !== null, set: (value: boolean) => { if (!value) dialog.value = null; } });
const current = computed(() => dialog.value !== null && props.model.current(dialog.value));
const labels = computed(() => tm('controller.proposals.labels') as MessageSchema['controller']['proposals']['labels']);
function choose(): void { props.model.select(family.value, targetId.value); }
async function load(kind: 'action' | 'event', id: string): Promise<void> { if (props.model.kind.value !== kind || props.model.id.value !== id) props.model.select(kind, id); await props.model.load(); }
async function request(operation: ProposalConfirmation['operation']): Promise<void> {
  const capture = props.model.prepare(operation);
  if (!capture) return;
  if (operation !== 'remove') {
    if (props.model.current(capture)) await props.model.confirm(capture);
  } else dialog.value = capture;
}
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
      {{ model.error.value === 'proposal_base_changed' ? labels.baseChanged : model.error.value }}
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
      <ControllerProposalBuilder
        :model="model"
        :automation="automation"
      />
      <div class="actions">
        <button
          :disabled="model.busy.value"
          @click="model.load()"
        >
          {{ labels.load }}
        </button><button
          v-if="model.canReadShared.value"
          @click="model.loadShared()"
        >
          {{ labels.shared }}
        </button><button
          :disabled="model.busy.value"
          @click="model.inspect()"
        >
          {{ labels.inspect }}
        </button>
      </div>
      <div
        v-if="model.sharedSource.value"
        class="status"
      >
        <p>{{ labels.sharedHint }}</p>
        <pre>{{ model.sharedSource.value.text }}</pre>
        <button
          :disabled="model.sharedSource.value.before !== model.text.value || !model.canReadShared.value"
          @click="model.useShared()"
        >
          {{ labels.useShared }}
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
      <p>{{ labels.limit }}</p><p>{{ labels.saveNote }}</p><p role="status">
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
          :disabled="!model.canSubmit.value"
          @click="model.submit(false)"
        >
          {{ model.matches.value ? labels.send : labels.saveSend }}
        </button>
        <button
          v-if="model.canSubmitActive.value"
          @click="model.submit(true)"
        >
          {{ model.matches.value ? labels.sendActive : labels.saveActive }}
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
