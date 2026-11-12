<script setup lang="ts">
import { useControllerLabels } from '../lib/use-controller-labels.ts';
import { useControllerPairing } from '../model/use-controller-pairing.ts';
import { useControllerWorkflowDrafts } from '../model/use-controller-workflow-drafts.ts';
import ControllerCommandJournal from './ControllerCommandJournal.vue';
import ControllerConnections from './ControllerConnections.vue';
import ControllerPointsPanel from './ControllerPointsPanel.vue';
import ControllerWorkflowPanel from './ControllerWorkflowPanel.vue';

import { computed, inject, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { sessionRecoveryKey } from '@/shared/api/session-recovery.ts';
import { createControllerModel, type PendingCommand, type ProfileForgetTarget, type RemoteProfile, type ReviewTarget, type WorkflowOperation } from '../model/controller.ts';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import SessionRecoveryDialog from '@/shared/ui/dialog/SessionRecoveryDialog.vue';
import ControllerAutomations from './ControllerAutomations.vue';
import ControllerProposals from './ControllerProposals.vue';
import ControllerLiveStatus from './ControllerLiveStatus.vue';
import ControllerLiveFeed from './ControllerLiveFeed.vue';
import ControllerCompetitions from './ControllerCompetitions.vue';
import type { LiveFeedEvent } from '../model/live-feed.ts';
import type { DraftSaveTarget } from '../model/drafts.ts';

const props = defineProps<{ browserMode?: boolean }>();
const control = useControlClient();
const recovery = inject(sessionRecoveryKey, null);
const model = createControllerModel(control);
const { status, points, pointsAllowed, error, busy, ready, commandSending } = model;
const { t } = useI18n();
const { labels, forgetLabels, cmd, dft } = useControllerLabels();
const phase = computed(() => labels.value[status.value.phase as 'idle' | 'pairing' | 'connecting' | 'ready' | 'reconnecting' | 'failed'] ?? status.value.phase);
// A lost browser session must offer recovery instead of a dead "failed"
// screen. The store retries quietly and only escalates to a dialog once its
// budget is spent or the cookie is provably invalid.
watch(() => [status.value.phase, status.value.errorCode] as const, ([current, code]) => {
  if (!props.browserMode || !recovery) return;
  if (current === 'ready') { recovery.report('ok'); return; }
  if (current !== 'failed' || !code) return;
  if (code === 'session_expired') { recovery.report('expired'); return; }
  if (['transport', 'connection_failed', 'host_unavailable', 'resync_required'].includes(code)) recovery.report('unavailable');
});
const snapshotBudgetHelp = computed(() => t('controller.root.snapshotBudgetHelp'));
const pointSearch = ref(''); const pointAmount = ref<string | number>('');
const pointsSection = ref<{ focus(): void } | null>(null);
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
  void nextTick(() => { pointsSection.value?.focus(); });
}
watch(() => points.value?.selectedId ?? null, () => { pointAmount.value = ''; });
type Confirmation = { kind:'draft';target:DraftSaveTarget;origin:string;hostId:string } | { kind:'review';target:ReviewTarget;intent:PendingCommand;origin:string;hostId:string } | { kind:'forget';target:ProfileForgetTarget };
const confirmation = ref<Confirmation | null>(null);
const storageReview = ref<string | null>(null);
const storageIdentity = () => JSON.stringify([status.value.epoch, status.value.origin, status.value.hello?.hostId, status.value.hello?.principalId, status.value.hello?.sessionId]);
const storageCurrent = computed(() => status.value.phase === 'ready' && status.value.browserJournalMissing === true && storageReview.value === storageIdentity());
async function initializeBrowserJournal(): Promise<void> {
  if (!storageCurrent.value) return;
  const epoch = status.value.epoch; storageReview.value = null;
  try { await control.call('controller.command.initialize', { epoch, acknowledgeStorageLoss: true }); }
  catch (failure) { error.value = failure instanceof Error ? failure.message : 'Command storage unavailable'; }
  await model.refresh();
}
const confirmOpen = computed({get:()=>confirmation.value!==null,set:(open:boolean)=>{if(!open)confirmation.value=null;}});
const confirmationCurrent = computed(()=>!!confirmation.value && (confirmation.value.kind==='draft' ? model.draftStorage.current(confirmation.value.target) : confirmation.value.kind==='forget' ? model.profileForgetCurrent(confirmation.value.target) : model.reviewIsCurrent(confirmation.value.target)));
function requestForget(profile: RemoteProfile): void {
  const target = model.prepareForget(profile);
  if (target) confirmation.value = { kind: 'forget', target };
}
function requestSaveDraft(): void {
  const target=model.draftStorage.prepare(selected.value);
  if(target && status.value.hello && status.value.origin)confirmation.value={kind:'draft',target,origin:status.value.origin,hostId:status.value.hello.hostId};
}
async function requestChange(operation:WorkflowOperation): Promise<void> {
  if(!row.value)return;const target=model.prepareCommand(row.value,operation);if(target)await model.changeWorkflow(target);
}
function requestReview(): void {
  const target=model.prepareReview();const journal=status.value.commandJournal;
  if(target&&journal?.pending)confirmation.value={kind:'review',target,intent:{...journal.pending},origin:journal.binding.origin,hostId:journal.binding.hostId};
}
async function confirmCommand(): Promise<void> {
  const item=confirmation.value;if(!item||!confirmationCurrent.value)return;confirmation.value=null;
  if(item.kind==='draft')await model.draftStorage.save(item.target);else if(item.kind==='forget')await model.forgetProfile(item.target);else await model.reviewUnknown(item.target);
}
watch(() => JSON.stringify([status.value.epoch, status.value.origin, status.value.hello?.connectionId,
  status.value.hello?.hostId, status.value.hello?.hostInstanceId, status.value.hello?.principalId,
  status.value.hello?.sessionId, status.value.hello?.grantRevision, status.value.hello?.capabilities,
  status.value.hello?.resourceRestrictions]), () => { confirmation.value=null; pointSearch.value=''; pointAmount.value=''; });
const workflowDrafts = useControllerWorkflowDrafts(model);
const { selected, row, draftBusy } = workflowDrafts;
const connection = useControllerPairing(control, model);
const { disconnect } = connection;
onMounted(() => { void model.start(props.browserMode ? 'browser' : window.__tiktools_controller__?.profile).then(() => {
  if (status.value.phase === 'pairing') connection.resume(status.value.pairingRequestId ?? '');
}); });
onUnmounted(() => { connection.stop(); model.stop(); });
</script>

<template>
  <main class="controller">
    <header
      class="controller__banner"
      aria-live="polite"
    >
      <div>
        <h1 class="controller__heading">
          {{ status.origin ? `${props.browserMode ? t('controller.root.viewing') : labels.controlling} ${status.origin}` : labels.title }}
        </h1><p class="controller__text">
          {{ phase }} <span v-if="status.errorCode">· {{ status.errorCode }}</span>
        </p><small v-if="status.hello">Host {{ status.hello.hostId }}</small>
      </div>
      <button
        v-if="status.phase !== 'idle'"
        class="controller__button"
        :disabled="busy"
        @click="disconnect"
      >
        {{ labels.disconnect }}
      </button>
    </header>
    <p
      v-if="status.errorCode === 'snapshot_rate_budget'"
      role="alert"
      class="controller__error controller__text"
    >
      {{ snapshotBudgetHelp }}
    </p>
    <p
      v-if="error"
      role="alert"
      class="controller__error controller__text"
    >
      {{ error }}
    </p>
    <ControllerCommandJournal
      :model="model"
      :browser-mode="props.browserMode"
      @review-storage="storageReview = storageIdentity()"
      @review-command="requestReview"
    />
    <ControllerConnections
      :model="model"
      :browser-mode="props.browserMode"
      :connection="connection"
      @forget="requestForget"
    />
    <ControllerLiveStatus :model="model.live" />
    <ControllerLiveFeed
      :model="model.liveFeed"
      :can-lookup-points="canLookupFeedViewer"
      :show-ambiguous-viewer="ready && pointsAllowed"
      @lookup-points="lookupFeedViewer"
    />
    <ControllerCompetitions :model="model.competitions" />
    <ControllerAutomations
      :model="model.automation"
      :epoch="status.epoch"
      :busy="busy || commandSending"
    />
    <ControllerProposals
      :model="model.proposals"
      :automation="model.automation"
    />
    <ControllerPointsPanel
      ref="pointsSection"
      v-model:search="pointSearch"
      v-model:amount="pointAmount"
      :model="model"
    />
    <ControllerWorkflowPanel
      :model="model"
      :browser-mode="props.browserMode"
      :drafts="workflowDrafts"
      @change="requestChange"
      @save-draft="requestSaveDraft"
    />
    <BaseConfirmDialog
      v-model:open="confirmOpen"
      :title="confirmation?.kind === 'draft' ? dft.title : confirmation?.kind === 'forget' ? forgetLabels.title : cmd.reviewTitle"
      :message="confirmation?.kind === 'draft' ? dft.consent : confirmation?.kind === 'forget' ? forgetLabels.message : cmd.reviewMessage"
      :confirm-label="confirmation?.kind === 'draft' ? dft.save : confirmation?.kind === 'forget' ? forgetLabels.action : cmd.reviewed"
      :busy="commandSending || busy || draftBusy || !confirmationCurrent"
      :danger="confirmation?.kind !== 'draft'"
      @confirm="confirmCommand"
    >
      <template v-if="confirmation?.kind === 'forget'">
        <p class="controller__text">
          {{ confirmation.target.name }}
        </p><code class="controller__code">{{ confirmation.target.origin }}</code><code class="controller__code">Host {{ confirmation.target.hostId }}</code>
      </template>
      <template v-else-if="confirmation?.kind === 'draft'">
        <code class="controller__code">{{ confirmation.origin }}</code><code class="controller__code">Host {{ confirmation.hostId }}</code>
        <p class="controller__text">
          {{ confirmation.target.id }} · {{ dft.base }} {{ confirmation.target.revision }}
        </p>
      </template>
      <template v-else-if="confirmation?.kind === 'review'">
        <code class="controller__code">{{ confirmation.origin }}</code><code class="controller__code">Host {{ confirmation.hostId }}</code>
        <p class="controller__text">
          {{ confirmation.intent.resourceId }}
        </p>
        <p
          v-if="confirmation.intent.delta !== undefined"
          class="controller__text"
        >
          {{ cmd.requested }}: {{ confirmation.intent.delta }} {{ confirmation.intent.currencyName }}
        </p>
        <code class="controller__code">{{ confirmation.intent.commandId }}</code>
      </template>
      <p
        v-if="confirmation && !confirmationCurrent"
        class="controller__text"
      >
        {{ cmd.changed }}
      </p>
    </BaseConfirmDialog>
    <BaseConfirmDialog
      :open="storageReview !== null"
      :title="t('controller.root.journalMissing.title')"
      :message="t('controller.root.journalMissing.message')"
      :confirm-label="t('controller.root.journalMissing.confirm')"
      :busy="!storageCurrent || commandSending || busy"
      danger
      @update:open="(open: boolean) => { if (!open) storageReview = null; }"
      @confirm="initializeBrowserJournal"
    />
  </main>
  <SessionRecoveryDialog />
</template>

<style src="./controller-root.css"></style>
