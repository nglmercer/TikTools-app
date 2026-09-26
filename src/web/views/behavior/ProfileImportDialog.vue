<script lang="tsx">
import { computed, ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import type { JsonObject } from '../../../automation/types.ts';
import { Button } from '../../components/ui/Button.vue';
import { FormField } from '../../components/ui/FormField.vue';
import { Modal, ModalActions } from '../../components/ui/Modal.vue';
import { SchemaForm } from '../../components/ui/SchemaForm.vue';
import { t, type Locale } from '../../i18n.ts';
import type { AppliedRuleTemplate } from './rule-templates.ts';
import { instantiateProfile, parseRuleProfileImport, profileParamSchema, type RuleProfile } from '../../features/rule-profiles.ts';
import { ImportDropzone } from './ImportDropzone.vue';
import { StagedTemplateList } from './StagedTemplateList.vue';

export type ProfileImportDialogProps = {
  locale: Locale;
  profilesError: string;
  onClose: () => void;
  onApplyProfile: (profile: RuleProfile, applied: AppliedRuleTemplate[]) => Promise<void>;
};

export const ProfileImportDialog = defineVueComponent<ProfileImportDialogProps>(
  ['locale', 'profilesError', 'onClose', 'onApplyProfile'],
  (props) => {
    const staged = ref<{ source: string; profile: RuleProfile } | null>(null);
    const errors = ref<string[]>([]);
    const importing = ref(false);
    const paramValues = ref<JsonObject>({});

    const collected = computed(() =>
      staged.value ? profileParamSchema(staged.value.profile) : null,
    );

    /** Profiles stage as one pack; raw JSON is never typed into the UI. */
    const stageContent = (text: string, source: string): void => {
      const profiled = parseRuleProfileImport(text);
      if (!profiled.profile) {
        staged.value = null;
        errors.value = [t(props.locale, 'behavior.copy.proImportNotProfile')];
        return;
      }
      staged.value = { source, profile: profiled.profile };
      // Every staged file starts from its own declared values — never from
      // a previous import's edits.
      paramValues.value = { ...(profileParamSchema(profiled.profile)?.defaults ?? {}) };
      errors.value = [];
    };

    const clearStaged = (): void => {
      staged.value = null;
      paramValues.value = {};
    };

    const runImport = async (): Promise<void> => {
      if (importing.value) return;
      const pending = staged.value;
      if (!pending) {
        errors.value = [t(props.locale, 'behavior.copy.tplImportEmpty')];
        return;
      }
      importing.value = true;
      try {
        // Profiles apply immediately (records + pack registration) with the
        // dialog's params winning over every layer; per-entry options stay
        // a CLI feature.
        await props.onApplyProfile(
          pending.profile,
          instantiateProfile(pending.profile, collected.value ? { ...paramValues.value } : {}),
        );
        props.onClose();
      } catch {
        // Failures surface through profilesError; the dialog stays open.
      } finally {
        importing.value = false;
      }
    };

    return () => {
      const locale = props.locale;
      const pending = staged.value;
      return (
        <Modal
          title={t(locale, 'behavior.copy.proImportTitle')}
          description={t(locale, 'behavior.copy.proImportLead')}
          size="lg"
          onClose={props.onClose}
          footer={
            <ModalActions>
              <Button variant="soft" onClick={() => props.onClose()}>
                {t(locale, 'cancel')}
              </Button>
              <Button variant="primary" loading={importing.value} onClick={() => void runImport()}>
                {pending
                  ? t(locale, 'behavior.copy.proApplyStaged', { count: pending.profile.templates.length })
                  : t(locale, 'behavior.copy.tplDoImport')}
              </Button>
            </ModalActions>
          }
        >
          <div class="template-config">
            <ImportDropzone
              locale={locale}
              title={t(locale, 'behavior.copy.proDropTitle')}
              hint={t(locale, 'behavior.copy.tplDropHint')}
              browseLabel={t(locale, 'behavior.copy.tplBrowse')}
              pasteLabel={t(locale, 'behavior.copy.tplPaste')}
              clipboardSource={t(locale, 'behavior.copy.tplClipboardSource')}
              pastePressKeys={t(locale, 'behavior.copy.tplPastePressKeys')}
              unreadableMessage={t(locale, 'behavior.copy.tplImportUnreadable')}
              onContent={(text, source) => stageContent(text, source)}
              onReadError={(message) => {
                staged.value = null;
                errors.value = [message];
              }}
            />
            {pending && (
              <StagedTemplateList
                locale={locale}
                headline={t(locale, 'behavior.copy.proStaged', { name: pending.profile.name, count: pending.profile.templates.length })}
                templates={pending.profile.templates.map((entry) => entry.template)}
                source={pending.source}
                stagedFrom={t(locale, 'behavior.copy.tplStagedFrom', { source: pending.source })}
                clearLabel={t(locale, 'behavior.copy.tplClearStaged')}
                hint={collected.value ? undefined : t(locale, 'behavior.copy.proApplyHint')}
                onClear={() => clearStaged()}
              />
            )}
            {pending && collected.value && (
              <div>
                <FormField label={t(locale, 'behavior.copy.proParamsLabel')}>
                  <SchemaForm
                    locale={locale}
                    schema={collected.value.schema}
                    value={paramValues.value}
                    onChange={(next) => { paramValues.value = next; }}
                  />
                </FormField>
                <p class="template-hint">{t(locale, 'behavior.copy.proParamsHint')}</p>
              </div>
            )}
            {props.profilesError ? <p class="template-error" role="alert">{props.profilesError}</p> : null}
            {errors.value.map((message) => (
              <p class="template-error" role="alert" key={message}>{message}</p>
            ))}
          </div>
        </Modal>
      );
    };
  },
);

export default ProfileImportDialog;
</script>
