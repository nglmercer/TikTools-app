<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { Modal, ModalActions } from './Modal.vue';
import { NumberInput } from './NumberInput.vue';
import { TextInput } from './TextInput.vue';
import { InfoTip } from './InfoTip.vue';

/** One param with labels pre-resolved: flat on purpose, so the props stay simple. */
export type TemplateParamField = {
  key: string;
  label: string;
  hint?: string;
  kind: 'text' | 'number';
  default: string;
  min?: number;
  max?: number;
  step?: number;
};

/** One answered param. Arrays cross component props; records stay local. */
export type TemplateParamAnswer = {
  key: string;
  value: string;
};

type TemplateConfigModalProps = {
  title: string;
  description: string;
  /** Params in template order; the form renders one typed field each. */
  params: TemplateParamField[];
  applyLabel: string;
  cancelLabel: string;
  onApply: (answers: TemplateParamAnswer[]) => void;
  onClose: () => void;
};

/**
 * Generic config form for a condition template's params: one typed field
 * per param, nothing template-specific. Numbers go through NumberInput
 * (with the param's bounds); everything else is plain text. Answers start
 * from the defaults; an emptied field restores its default at apply time
 * (see `applyConditionTemplate`), so clearing can never store a value that
 * never matches.
 */
export const TemplateConfigModal = defineVueComponent<TemplateConfigModalProps>(
  ['title', 'description', 'params', 'applyLabel', 'cancelLabel', 'onApply', 'onClose'],
  (props) => {
  const seed: Record<string, string> = {};
  for (const param of props.params) seed[param.key] = param.default;
  const answers = ref<Record<string, string>>(seed);

  const setAnswer = (key: string, value: string): void => {
    answers.value = { ...answers.value, [key]: value };
  };

  const numberValue = (key: string): number | null => {
    const raw = (answers.value[key] ?? '').trim();
    if (raw === '') return null;
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? parsed : null;
  };

  return () => (
    <Modal
      title={props.title}
      description={props.description}
      onClose={props.onClose}
      closeLabel={props.cancelLabel}
      footer={(
        <ModalActions>
          <button type="button" class="plg-btn plg-btn--sm" onClick={props.onClose}>{props.cancelLabel}</button>
          <button
            type="button"
            class="plg-btn plg-btn--primary plg-btn--sm"
            onClick={() => props.onApply(props.params.map((param) => ({ key: param.key, value: answers.value[param.key] ?? '' })))}
          >
            {props.applyLabel}
          </button>
        </ModalActions>
      )}
    >
      <div class="ui-template-config">
        {props.params.map((param) => (
          <div class="plg-field" key={param.key}>
            <div class="plg-label-row">
              <span class="plg-label">{param.label}</span>
              {param.hint && <InfoTip text={param.hint} position="right" />}
            </div>
            {param.kind === 'number' ? (
              <NumberInput
                value={numberValue(param.key)}
                onValueChange={(next) => setAnswer(param.key, next === null ? '' : String(next))}
                min={param.min}
                max={param.max}
                step={param.step}
              />
            ) : (
              <TextInput
                value={answers.value[param.key] ?? ''}
                onValueChange={(next) => setAnswer(param.key, next)}
              />
            )}
          </div>
        ))}
      </div>
    </Modal>
  );
  },
);

export default TemplateConfigModal;
</script>
