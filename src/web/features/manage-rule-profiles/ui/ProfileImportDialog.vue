<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JsonObject } from '../../../../automation/types.ts';
import type { AppliedRuleTemplate } from '@/entities/rule-template/lib/rule-templates.ts';
import {
  instantiateProfile,
  parseRuleProfileImport,
  profileParamSchema,
  type RuleProfile,
} from '@/entities/rule-template/model/rule-profiles-state.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseFileDrop from '@/shared/ui/file-drop/BaseFileDrop.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
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
const staged = ref<{ source: string; profile: RuleProfile } | null>(null);
const problems = ref<string[]>([]);
const values = ref<JsonObject>({});
const candidate = ref<AppliedRuleTemplate[] | null>(null);
watch(values, () => { if (!props.working) candidate.value = null; }, { deep: true });

const settings = computed(() => (staged.value ? profileParamSchema(staged.value.profile) : null));
const messages = computed(() => [...problems.value, ...(props.error ? [props.error] : [])]);

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
  <BaseDialog
    v-model:open="open"
    :title="t('automations.import.profileTitle')"
    :width="600"
  >
    <p class="profile-import__lead">
      {{ t('automations.import.profileLead') }}
    </p>
    <BaseFileDrop
      :title="t('automations.import.dropTitle')"
      :hint="t('automations.import.dropHint')"
      :browse-label="t('automations.import.browse')"
      :paste-label="t('automations.import.paste')"
      :paste-blocked-label="t('automations.import.pasteBlocked')"
      :clipboard-label="t('automations.import.clipboard')"
      :unreadable-label="t('automations.import.unreadable')"
      :disabled="working"
      @content="stage"
      @error="onError"
    />
    <div
      v-if="staged"
      class="profile-import__staged"
    >
      <p>
        <strong>{{ t('automations.import.profileStaged', { name: staged.profile.name, count: staged.profile.templates.length }) }}</strong>
        {{ t('automations.import.from', { source: staged.source }) }}
      </p>
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
    </div>
    <p
      v-for="message in messages"
      :key="message"
      class="profile-import__error"
      role="alert"
    >
      {{ message }}
    </p>
    <template #footer>
      <BaseButton
        variant="primary"
        :disabled="!staged || working"
        @click="apply"
      >
        {{ staged ? t('automations.import.profileApply', { count: staged.profile.templates.length }) : t('automations.import.run') }}
      </BaseButton>
      <BaseButton
        :disabled="working"
        @click="open = false"
      >
        {{ t('common.cancel') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.profile-import__lead {
  margin: 0 0 var(--space-4);
  color: var(--color-text-muted);
}

.profile-import__staged {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  margin-top: var(--space-4);
}

.profile-import__staged p {
  margin: 0;
}

.profile-import__error {
  margin: var(--space-3) 0 0;
  color: var(--color-danger-hover);
}
</style>
