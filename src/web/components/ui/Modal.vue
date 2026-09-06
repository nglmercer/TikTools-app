<script lang="tsx">
import { onMounted, onUnmounted, ref } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import { Button } from './Button.vue';
import { FormField } from './FormField.vue';
import { TextInput, type TextInputHandle } from './TextInput.vue';

export type ModalProps = {
  title: string;
  description?: string;
  children?: VNodeChild;
  footer?: VNodeChild;
  onClose: () => void;
  closeLabel?: string;
  closeOnBackdrop?: boolean;
  className?: string;
};

/**
 * Small, application-owned dialog primitive. Keeping this outside the
 * automation view makes prompts and confirmations behave consistently across
 * the editor and the rest of the WebView UI.
 */
export const Modal = defineVueComponent<ModalProps>(
  ['title', 'description', 'children', 'footer', 'onClose', 'closeLabel', 'closeOnBackdrop', 'className'],
  (props) => {
  const dialogRef = ref<HTMLDivElement | null>(null);

  onMounted(() => {
    const previousFocus = document.activeElement as HTMLElement | null;
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      props.onClose();
    };

    document.addEventListener('keydown', handleKeyDown);
    const firstField = dialogRef.value?.querySelector<HTMLElement>(
      'input:not([disabled]), textarea:not([disabled]), select:not([disabled]), button:not(.ui-modal__close):not([disabled])',
    );
    (firstField ?? dialogRef.value)?.focus();

    onUnmounted(() => {
      document.removeEventListener('keydown', handleKeyDown);
      if (previousFocus?.isConnected) previousFocus.focus();
    });
  });

  return () => {
    const { title, description, children, footer, onClose, closeLabel = 'Close', closeOnBackdrop = true, className = '' } = props;
    return (
    <div
      class="ui-modal-backdrop"
      role="presentation"
      onMousedown={(event) => {
        if (closeOnBackdrop && event.target === event.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        class={`ui-modal-card ${className}`.trim()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="ui-modal-title"
        tabindex={-1}
        onMousedown={(event) => event.stopPropagation()}
      >
        <header class="ui-modal-card__header">
          <h2 id="ui-modal-title" class="ui-modal-card__title">{title}</h2>
          <button
            type="button"
            class="ui-modal__close"
            aria-label={closeLabel}
            onClick={onClose}
          >
            ×
          </button>
        </header>
        {description ? <p class="ui-modal-card__description">{description}</p> : null}
        {children ? <div class="ui-modal-card__body">{children}</div> : null}
        {footer ? <footer class="ui-modal-card__footer">{footer}</footer> : null}
      </div>
    </div>
    );
  };
  },
);

export type TextPromptModalProps = {
  title: string;
  description?: string;
  label: string;
  initialValue?: string;
  placeholder?: string;
  confirmLabel: string;
  cancelLabel: string;
  requiredMessage?: string;
  closeLabel?: string;
  closeOnBackdrop?: boolean;
  onConfirm: (value: string) => void;
  onClose: () => void;
};

export const TextPromptModal = defineVueComponent<TextPromptModalProps>(
  ['title', 'description', 'label', 'initialValue', 'placeholder', 'confirmLabel', 'cancelLabel', 'requiredMessage', 'closeLabel', 'closeOnBackdrop', 'onConfirm', 'onClose'],
  (props) => {
  const value = ref(props.initialValue ?? '');
  const error = ref('');
  const inputRef = ref<TextInputHandle | null>(null);

  const confirm = (): void => {
    const nextValue = value.value.trim();
    if (props.requiredMessage && !nextValue) {
      error.value = props.requiredMessage;
      inputRef.value?.focus();
      return;
    }
    props.onConfirm(nextValue);
  };

  return () => {
    const { title, description, label, placeholder, confirmLabel, cancelLabel, closeLabel, closeOnBackdrop, onClose } = props;
    return (
    <Modal
      title={title}
      description={description}
      onClose={onClose}
      closeLabel={closeLabel}
      closeOnBackdrop={closeOnBackdrop}
      footer={
        <div class="ui-modal-card__actions">
        <Button variant="soft" onClick={onClose}>{cancelLabel}</Button>
          <Button variant="primary" onClick={confirm}>{confirmLabel}</Button>
        </div>
      }
      >
        <FormField label={label} error={error.value} required={Boolean(props.requiredMessage)}>
        <TextInput
          ref={inputRef}
          value={value.value}
          onValueChange={(nextValue) => {
            value.value = nextValue;
            if (error.value) error.value = '';
          }}
          placeholder={placeholder}
          required={Boolean(props.requiredMessage)}
          onEnter={confirm}
          spellCheck={false}
        />
      </FormField>
    </Modal>
    );
  };
  },
);

export type AlertModalProps = {
  title: string;
  description?: string;
  okLabel: string;
  closeLabel?: string;
  closeOnBackdrop?: boolean;
  onClose: () => void;
};

export function AlertModal({
  title,
  description,
  okLabel,
  closeLabel,
  closeOnBackdrop,
  onClose,
}: AlertModalProps) {
  return (
    <Modal
      title={title}
      description={description}
      onClose={onClose}
      closeLabel={closeLabel}
      closeOnBackdrop={closeOnBackdrop}
      footer={
        <div class="ui-modal-card__actions">
          <Button variant="primary" onClick={onClose}>{okLabel}</Button>
        </div>
      }
    />
  );
}

export type ConfirmModalProps = {
  title: string;
  description?: string;
  confirmLabel: string;
  cancelLabel: string;
  closeLabel?: string;
  closeOnBackdrop?: boolean;
  onConfirm: () => void;
  onClose: () => void;
  danger?: boolean;
};

export function ConfirmModal({
  title,
  description,
  confirmLabel,
  cancelLabel,
  closeLabel,
  closeOnBackdrop,
  onConfirm,
  onClose,
  danger = false,
}: ConfirmModalProps) {
  return (
    <Modal
      title={title}
      description={description}
      onClose={onClose}
      closeLabel={closeLabel}
      closeOnBackdrop={closeOnBackdrop}
      footer={
        <div class="ui-modal-card__actions">
          <Button variant="soft" onClick={onClose}>{cancelLabel}</Button>
          <Button variant={danger ? 'danger' : 'primary'} onClick={onConfirm}>{confirmLabel}</Button>
        </div>
      }
    />
  );
}

export default Modal;
</script>
