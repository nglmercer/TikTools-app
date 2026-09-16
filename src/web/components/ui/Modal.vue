<script lang="tsx">
import { onMounted, onUnmounted, ref, Teleport } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent, defineVueFunctional } from '../../vue/component.ts';

import { IconClose } from '../icons/index.ts';
import { Button } from './Button.vue';
import { FormField } from './FormField.vue';
import { TextInput, type TextInputHandle } from './TextInput.vue';
import {
  createModalIds,
  FOCUSABLE_SELECTOR,
  isBackdropDismiss,
  isEscapeDismiss,
  MODAL_SIZE_CLASS,
  trapFocusTarget,
  type ModalSize,
} from './modal-logic.ts';

export type { ModalSize };

export type ModalProps = {
  title: string;
  description?: string;
  children?: VNodeChild;
  footer?: VNodeChild;
  size?: ModalSize;
  onClose: () => void;
  closeLabel?: string;
  closeOnBackdrop?: boolean;
  closeOnEscape?: boolean;
  className?: string;
};

/** Shared footer actions row so confirm/alert/prompt modals stay consistent. */
export const ModalActions = defineVueFunctional<{ children?: VNodeChild }>((props) => (
  <div class="ui-modal-card__actions">{props.children}</div>
));

/**
 * Small, application-owned dialog primitive. Keeping this outside the
 * automation view makes prompts and confirmations behave consistently across
 * the editor and the rest of the WebView UI.
 */
export const Modal = defineVueComponent<ModalProps>(
  ['title', 'description', 'children', 'footer', 'size', 'onClose', 'closeLabel', 'closeOnBackdrop', 'closeOnEscape', 'className'],
  (props) => {
  const dialogRef = ref<HTMLDivElement | null>(null);
  const ids = createModalIds();

  onMounted(() => {
    const previousFocus = document.activeElement as HTMLElement | null;
    const handleKeyDown = (event: KeyboardEvent): void => {
      // With stacked modals only the topmost one handles keyboard dismissal.
      const backdrops = document.querySelectorAll('.ui-modal-backdrop');
      const topmost = backdrops[backdrops.length - 1];
      if (!topmost || (dialogRef.value && !topmost.contains(dialogRef.value))) return;

      if (isEscapeDismiss(props.closeOnEscape ?? true, event.key)) {
        event.preventDefault();
        props.onClose();
        return;
      }
      if (event.key !== 'Tab') return;
      const dialog = dialogRef.value;
      if (!dialog) return;
      const focusables = Array.from(dialog.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR))
        .filter((element) => element.getClientRects().length > 0);
      const target = trapFocusTarget(
        focusables.indexOf(document.activeElement as HTMLElement),
        focusables.length,
        event.shiftKey,
      );
      if (target !== null) {
        event.preventDefault();
        focusables[target]?.focus();
      }
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
    const { title, description, children, footer, size = 'md', onClose, closeLabel = 'Close', closeOnBackdrop = true, className = '' } = props;
    const sizeClass = MODAL_SIZE_CLASS[size];
    return (
    <Teleport to="body">
      <div
        class="ui-modal-backdrop"
        role="presentation"
        onMousedown={(event) => {
          if (isBackdropDismiss(closeOnBackdrop, event.target, event.currentTarget)) onClose();
        }}
      >
        <div
          ref={dialogRef}
          class={`ui-modal-card ${sizeClass} ${className}`.trim().replace(/\s+/g, ' ')}
          role="dialog"
          aria-modal="true"
          aria-labelledby={ids.titleId}
          aria-describedby={description ? ids.descriptionId : undefined}
          tabindex={-1}
          onMousedown={(event) => event.stopPropagation()}
        >
          <header class="ui-modal-card__header">
            <h2 id={ids.titleId} class="ui-modal-card__title">{title}</h2>
            <button
              type="button"
              class="ui-modal__close"
              aria-label={closeLabel}
              onClick={onClose}
            >
              <IconClose size={12} />
            </button>
          </header>
          {description ? <p id={ids.descriptionId} class="ui-modal-card__description">{description}</p> : null}
          {children ? <div class="ui-modal-card__body">{children}</div> : null}
          {footer ? <footer class="ui-modal-card__footer">{footer}</footer> : null}
        </div>
      </div>
    </Teleport>
    );
  };
  },
);

export type ModalVariantProps = {
  title: string;
  description?: string;
  size?: ModalSize;
  closeLabel?: string;
  closeOnBackdrop?: boolean;
  closeOnEscape?: boolean;
  onClose: () => void;
};

export type TextPromptModalProps = ModalVariantProps & {
  label: string;
  initialValue?: string;
  placeholder?: string;
  confirmLabel: string;
  cancelLabel: string;
  requiredMessage?: string;
  onConfirm: (value: string) => void;
};

export const TextPromptModal = defineVueComponent<TextPromptModalProps>(
  ['title', 'description', 'size', 'label', 'initialValue', 'placeholder', 'confirmLabel', 'cancelLabel', 'requiredMessage', 'closeLabel', 'closeOnBackdrop', 'closeOnEscape', 'onConfirm', 'onClose'],
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
    const { title, description, size, label, placeholder, confirmLabel, cancelLabel, closeLabel, closeOnBackdrop, closeOnEscape, onClose } = props;
    return (
    <Modal
      title={title}
      description={description}
      size={size}
      onClose={onClose}
      closeLabel={closeLabel}
      closeOnBackdrop={closeOnBackdrop}
      closeOnEscape={closeOnEscape}
      footer={
        <ModalActions>
          <Button variant="soft" onClick={onClose}>{cancelLabel}</Button>
          <Button variant="primary" onClick={confirm}>{confirmLabel}</Button>
        </ModalActions>
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

export type AlertModalProps = ModalVariantProps & {
  okLabel: string;
};

export function AlertModal({
  title,
  description,
  size,
  okLabel,
  closeLabel,
  closeOnBackdrop,
  closeOnEscape,
  onClose,
}: AlertModalProps) {
  return (
    <Modal
      title={title}
      description={description}
      size={size}
      onClose={onClose}
      closeLabel={closeLabel}
      closeOnBackdrop={closeOnBackdrop}
      closeOnEscape={closeOnEscape}
      footer={
        <ModalActions>
          <Button variant="primary" onClick={onClose}>{okLabel}</Button>
        </ModalActions>
      }
    />
  );
}

export type ConfirmModalProps = ModalVariantProps & {
  confirmLabel: string;
  cancelLabel: string;
  onConfirm: () => void;
  danger?: boolean;
};

export function ConfirmModal({
  title,
  description,
  size,
  confirmLabel,
  cancelLabel,
  closeLabel,
  closeOnBackdrop,
  closeOnEscape,
  onConfirm,
  onClose,
  danger = false,
}: ConfirmModalProps) {
  return (
    <Modal
      title={title}
      description={description}
      size={size}
      onClose={onClose}
      closeLabel={closeLabel}
      closeOnBackdrop={closeOnBackdrop}
      closeOnEscape={closeOnEscape}
      footer={
        <ModalActions>
          <Button variant="soft" onClick={onClose}>{cancelLabel}</Button>
          <Button variant={danger ? 'danger' : 'primary'} onClick={onConfirm}>{confirmLabel}</Button>
        </ModalActions>
      }
    />
  );
}

export default Modal;
</script>
