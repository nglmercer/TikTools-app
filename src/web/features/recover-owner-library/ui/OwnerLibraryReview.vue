<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { libraryCurrentPreview, libraryPreview } from '@/entities/rule-template/model/library-preview.ts';
import { libraryTarget } from '@/entities/rule-template/model/library-contracts.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconCompass from '@/shared/ui/icon/icons/IconCompass.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import type { ChangeReviewItem } from '../../review-shared/model/change-review.ts';
import ChangeReviewWorkspace from '../../review-shared/ui/ChangeReviewWorkspace.vue';
import TutorialCatalogDialog from '@/features/guided-help/ui/TutorialCatalogDialog.vue';

/**
 * The one review of an Owner library change. It is mounted once by the app shell, so an
 * import started from any page shows this dialog, and navigating keeps the pending
 * review: the request settles only when the owner confirms or discards it.
 */
const { t } = useI18n(); const journal = libraryJournal(useControlClient());
const guideOpen = ref(false);
const busy = computed(() => journal.busy.value || journal.gate.busy.value);
const draft = journal.draft;
const reviewOpen = computed({ get: () => !!draft.value, set: (value) => { if (!value) journal.dismiss(); } });
const planned = computed(() => draft.value ? libraryPreview(draft.value.context, draft.value.mutation) : []);
const current = computed(() => draft.value && journal.comparison.value ? libraryCurrentPreview(draft.value.context, journal.comparison.value, draft.value.mutation) : []);
const changedTemplates = computed(() => {
  const mutation = draft.value?.mutation;
  if (!mutation) return [];
  if (mutation.operation === 'delete-template') return draft.value!.context.catalog.templates.filter((entry) => entry.id === mutation.id).map((before) => ({ before, after: null }));
  if (mutation.operation === 'import-templates' || mutation.operation === 'apply-template' || mutation.operation === 'import-profile') return (mutation.templates ?? []).map((after) => ({ before: draft.value!.context.catalog.templates.find((entry) => entry.id === after.id) ?? null, after }));
  return [];
});
const descriptor = computed(() => {
  const mutation = draft.value?.mutation;
  if (!mutation) return '';
  if (mutation.operation === 'create-profile') return mutation.name;
  if (mutation.operation === 'import-profile') return mutation.profile.name;
  if (mutation.operation === 'import-templates') return mutation.templates.map((entry) => typeof entry.title === 'string' ? entry.title : entry.title.default).join(', ');
  const id = mutation.operation === 'assign-record' ? mutation.profileId : libraryTarget(mutation);
  return draft.value!.context.catalog.packs.find((pack) => pack.id === id)?.name ?? id;
});

const isSwitchProfile = computed(() => draft.value?.mutation.operation === 'switch-profile');

/** Records that leave the active profile: enabled before, disabled after. */
function isLeaving(before: unknown, after: unknown): boolean {
  if (!isSwitchProfile.value) return false;
  const was = (before as { enabled?: boolean } | null)?.enabled === true;
  const will = (after as { enabled?: boolean } | null)?.enabled === true;
  return was && !will;
}

function titleOf(value: { title?: unknown; id?: string } | null | undefined): string {
  const title = value?.title;
  if (typeof title === 'string') return title;
  if (title && typeof title === 'object' && 'default' in title && typeof (title as { default?: unknown }).default === 'string') {
    return (title as { default: string }).default;
  }
  return value?.id ?? '';
}

/**
 * The rows the owner reviews. When the host moved since the review started and
 * the owner re-reads it, the plan re-bases on the current host state so the
 * cards always answer "what changes if I confirm now". For a profile switch
 * the diff is mostly the enabled flag, which is exactly what the element view
 * surfaces; records outside the new profile are filterable, not deleted.
 */
const items = computed<ChangeReviewItem[]>(() => {
  const rows = current.value.length ? current.value : planned.value;
  const records = rows.map((row, index) => {
    let after = row.after;
    // Extra host records unknown to a frozen profile-switch plan are simply
    // disabled when the switch applies; they are not deleted.
    if (!after && row.before && isSwitchProfile.value) after = { ...row.before, enabled: false };
    // Assigning a record to a profile leaves the record itself untouched:
    // the change is the membership, so the card is pinned as modified and
    // the full record stays visible as context instead of collapsing away.
    const membership = draft.value?.mutation.operation === 'assign-record';
    return {
      id: `planned-${index}`,
      title: row.name,
      subtitle: row.kind ? t(`jsonView.reviewTable.${row.kind}`) : '',
      status: membership ? 'modified' as const : undefined,
      before: row.before ?? undefined,
      after: after ?? undefined,
      leaving: isLeaving(row.before, after),
      editable: row.before === null && row.after !== null,
      alwaysShowFields: membership,
    };
  });
  const templates = changedTemplates.value.map((entry, index) => ({
    id: `template-${index}`,
    title: titleOf(entry.after ?? entry.before),
    before: entry.before ?? undefined,
    after: entry.after ?? undefined,
  }));
  return [...records, ...templates];
});

function onRename(id: string, name: string): void {
  const match = /^planned-(\d+)$/.exec(id);
  if (!match) return;
  const row = planned.value[Number(match[1])];
  if (!row?.after) return;
  journal.renameRecord(row.kind, row.after.id, name);
}
</script>

<template>
  <BaseDialog
    v-model:open="reviewOpen"
    :title="t('libraryRecovery.reviewTitle')"
    :width="900"
    :closable="!busy"
    class="library-changes"
  >
    <template #header-actions>
      <BaseIconButton
        :label="t('jsonView.reviewTable.docs')"
        variant="default"
        data-test="library-review-guide"
        @click="guideOpen = true"
      >
        <IconCompass />
      </BaseIconButton>
    </template>
    <template v-if="draft">
      <p class="library-changes__operation">
        <strong>{{ t(`libraryRecovery.${draft.mutation.operation}`) }} · {{ descriptor }}</strong>
      </p>
      <p
        v-if="draft.error"
        role="alert"
      >
        {{ draft.error }}
      </p>
      <p
        v-if="draft.reviewRequired || draft.submitted"
        role="alert"
      >
        {{ t(draft.submitted ? 'libraryRecovery.noResend' : 'libraryRecovery.changed') }}
      </p>
      <ChangeReviewWorkspace
        :items="items"
        :profile-filter="isSwitchProfile"
        data-test="library-review-records"
        @rename="onRename"
      />
    </template>
    <template #footer>
      <BaseIconButton
        v-if="draft && (draft.reviewRequired || draft.submitted)"
        :label="t('libraryRecovery.readCurrent')"
        variant="default"
        :disabled="busy || journal.reading.value"
        data-test="library-read-current"
        @click="journal.readForReview"
      >
        <IconRefresh />
      </BaseIconButton>
      <BaseIconButton
        v-if="journal.comparison.value && !draft?.submitted"
        :label="t('libraryRecovery.useCurrent')"
        variant="default"
        :disabled="busy"
        data-test="library-use-current"
        @click="journal.useReviewed"
      >
        <IconUndo />
      </BaseIconButton>
      <BaseIconButton
        :label="t('libraryRecovery.confirm')"
        variant="primary"
        :disabled="!draft || busy || draft.reviewRequired || draft.submitted"
        data-test="library-confirm"
        @click="journal.confirm"
      >
        <IconCheck />
      </BaseIconButton>
      <BaseButton
        :disabled="busy"
        data-test="library-dismiss"
        @click="journal.dismiss"
      >
        {{ t(draft?.submitted ? 'libraryRecovery.dismiss' : 'common.cancel') }}
      </BaseButton>
    </template>
  </BaseDialog>
  <TutorialCatalogDialog
    v-model:open="guideOpen"
    :startable="false"
  />
</template>

<style scoped>
.library-changes :deep(.base-dialog__body) { display: grid; gap: 12px; min-width: 0; align-content: start; }
.library-changes__operation { margin: 0; }
</style>
