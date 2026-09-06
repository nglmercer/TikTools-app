<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import type { JsonObject } from '../../../automation/types.ts';
import type { Locale } from '../../i18n.ts';
import type { OpenMediaPicker } from '../../../shared/messages.ts';
import { Button } from './Button.vue';
import { Modal } from './Modal.vue';
import { SchemaForm } from './SchemaForm.vue';
import type { AutocompleteItem } from '../autocomplete/index.ts';

export type SchemaModalProps = {
  locale: Locale;
  title: string;
  description?: string;
  schema: JsonObject;
  uiHints?: JsonObject;
  initialValue: JsonObject;
  cancelLabel: string;
  applyLabel: string;
  templateSuggestions?: AutocompleteItem[];
  onApply: (value: JsonObject) => void;
  onClose: () => void;
  onOpenMediaPicker?: OpenMediaPicker;
};

/** Modal shell for any host-owned JSON Schema form, including plugin settings. */
export const SchemaModal = defineVueComponent<SchemaModalProps>(
  ['locale', 'title', 'description', 'schema', 'uiHints', 'initialValue', 'cancelLabel', 'applyLabel', 'templateSuggestions', 'onApply', 'onClose', 'onOpenMediaPicker'],
  (props) => {
  const value = ref<JsonObject>({ ...props.initialValue });
  return () => {
    const { locale, title, description, schema, uiHints, cancelLabel, applyLabel, templateSuggestions, onApply, onClose } = props;
    return (
    <Modal
      title={title}
      description={description}
      class="ui-modal-card--wide"
      onClose={onClose}
      footer={
        <div class="ui-modal-card__actions">
          <Button variant="soft" onClick={onClose}>{cancelLabel}</Button>
          <Button variant="primary" onClick={() => onApply(value.value)}>{applyLabel}</Button>
        </div>
      }
    >
      <SchemaForm locale={locale} schema={schema} uiHints={uiHints} value={value.value} onChange={(next) => (value.value = next)} templateSuggestions={templateSuggestions} onOpenMediaPicker={props.onOpenMediaPicker} />
    </Modal>
    );
  };
  },
);

export default SchemaModal;
</script>
