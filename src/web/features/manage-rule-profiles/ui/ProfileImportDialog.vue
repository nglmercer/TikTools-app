<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JsonObject } from '../../../../automation/types.ts';
import type { AppliedRuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import {
  instantiateProfile,
  MAX_PROFILE_TEXT_BYTES,
  parseRuleProfileImport,
  profileParamSchema,
  type RuleProfile,
} from '@/entities/rule-template/lib/rule-profiles.ts';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseImportDialog from '@/shared/ui/import-dialog/BaseImportDialog.vue';
import type { ImportTab, StagedImportItem } from '@/shared/ui/import-dialog/import-types.ts';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import { ControlCallError } from '@/shared/api/control-client.ts';

const props = withDefaults(
  defineProps<{
    error?: string | null;
    working?: boolean;
  }>(),
  { error: null, working: false },
);

const emit = defineEmits<{ apply: [profile: RuleProfile, applied: AppliedRuleTemplate[]] }>();
const open = defineModel<boolean>('open', { default: false });

const { t } = useI18n();
/** Profiles come from a file or the clipboard; the Cloud catalog imports them from Plugins, Explore. */
const tab = ref('manual');
const tabs = computed<ImportTab[]>(() => [{ value: 'manual', label: t('importDialog.manualTab'), icon: IconUpload }]);
const staged = ref<{ source: string; profile: RuleProfile } | null>(null);
const problems = ref<string[]>([]);
const values = ref<JsonObject>({});
const candidate = ref<AppliedRuleTemplate[] | null>(null);
watch(values, () => { if (!props.working) candidate.value = null; }, { deep: true });

const settings = computed(() => (staged.value ? profileParamSchema(staged.value.profile) : null));
const messages = computed(() => [...problems.value, ...(props.error ? [props.error] : [])]);
const stagedItems = computed<StagedImportItem[]>(() =>
  staged.value
    ? [{
        key: staged.value.profile.id,
        label: t('automations.import.profileStaged', { name: staged.value.profile.name, count: staged.value.profile.templates.length }),
      }]
    : [],
);
const labels = computed(() => ({
  dropTitle: t('automations.import.dropTitle'),
  dropHint: t('automations.import.dropHint'),
  browse: t('automations.import.browse'),
  paste: t('automations.import.paste'),
  pasteBlocked: t('automations.import.pasteBlocked'),
  clipboard: t('automations.import.clipboard'),
  unreadable: t('automations.import.unreadable'),
  staged: t('automations.import.staged', { count: stagedItems.value.length }),
  clear: t('automations.import.clear'),
  close: t('common.cancel'),
}));
const importLabel = computed(() =>
  staged.value ? t('automations.import.profileApply', { count: staged.value.profile.templates.length }) : t('automations.import.run'),
);

watch(open, (value) => {
  if (!value) return;
  staged.value = null;
  problems.value = [];
  values.value = {};
});

function stage(text: string, source: string): void {
  candidate.value = null;
  const parsed = parseRuleProfileImport(text);
  if (!parsed.profile) {
    staged.value = null;
    problems.value = [t('automations.import.notProfile')];
    return;
  }
  staged.value = { source, profile: parsed.profile };
  values.value = { ...(profileParamSchema(parsed.profile)?.defaults ?? {}) };
  problems.value = [];
}

function onError(message: string): void {
  staged.value = null;
  problems.value = [message];
}

function clear(): void {
  staged.value = null;
  candidate.value = null;
  problems.value = [];
}

function apply(): void {
  const pending = staged.value;
  if (!pending || props.working) {
    problems.value = [t('automations.import.empty')];
    return;
  }
  try { candidate.value ??= instantiateProfile(pending.profile, settings.value ? { ...values.value } : {}); }
  catch (failure) { problems.value = [t(failure instanceof ControlCallError && failure.code === 'too_large' ? 'libraryRecovery.expansionTooLarge' : 'libraryRecovery.expansionInvalid')]; return; }
  problems.value = [];
  emit('apply', pending.profile, candidate.value);
}
</script>

<template>
  <BaseImportDialog
    v-model:open="open"
    v-model:tab="tab"
    :title="t('automations.import.profileTitle')"
    :width="600"
    :tabs="tabs"
    :max-bytes="MAX_PROFILE_TEXT_BYTES"
    :staged="stagedItems"
    :staged-source="staged?.source"
    :busy="working"
    :errors="messages"
    :import-label="importLabel"
    :labels="labels"
    @content="stage"
    @error="onError"
    @clear="clear"
    @import="apply"
  >
    <template #manual-lead>
      <p class="profile-import__lead">
        {{ t('automations.import.profileLead') }}
      </p>
    </template>
    <template #staged-extra>
      <BaseGroupBox
        v-if="settings"
        :title="t('automations.import.profileSettings')"
        :description="t('automations.import.profileSettingsHint')"
      >
        <BaseSchemaForm
          v-model="values"
          :schema="settings.schema"
        />
      </BaseGroupBox>
    </template>
  </BaseImportDialog>
</template>

<style scoped>
.profile-import__lead {
  margin: 0;
  color: var(--color-text-muted);
}
</style>
