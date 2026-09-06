<script lang="tsx">
import { defineVueComponent } from '../../vue/component.ts';
import { t, type Locale } from '../../i18n.ts';
import {
  AlertModal,
  ConfirmModal,
  TextPromptModal,
} from './Modal.vue';
import { completeDialog, dialogQueue, type DialogRequest } from '../../composables/useDialogs.ts';

type DialogHostProps = {
  locale: Locale;
};

export const DialogHost = defineVueComponent<DialogHostProps>(['locale'], (props) => {
  const closeOnBackdrop = (request: DialogRequest): boolean => request.options.closeOnBackdrop ?? false;

  return () => {
    const request = dialogQueue[0];
    if (!request) return null;

    const title = request.options.title
      ?? t(props.locale, request.kind === 'alert' ? 'dialogAlertTitle' : request.kind === 'confirm' ? 'dialogConfirmTitle' : 'dialogPromptTitle');
    const closeLabel = request.options.closeLabel ?? t(props.locale, 'dialogClose');

    if (request.kind === 'alert') {
      return (
        <AlertModal
          key={request.id}
          title={title}
          description={request.message}
          okLabel={request.options.okLabel ?? t(props.locale, 'dialogOk')}
          closeLabel={closeLabel}
          closeOnBackdrop={closeOnBackdrop(request)}
          onClose={() => completeDialog(request, undefined)}
        />
      );
    }

    if (request.kind === 'confirm') {
      return (
        <ConfirmModal
          key={request.id}
          title={title}
          description={request.message}
          confirmLabel={request.options.confirmLabel ?? t(props.locale, 'dialogConfirm')}
          cancelLabel={request.options.cancelLabel ?? t(props.locale, 'dialogCancel')}
          closeLabel={closeLabel}
          closeOnBackdrop={closeOnBackdrop(request)}
          danger={request.options.danger}
          onConfirm={() => completeDialog(request, true)}
          onClose={() => completeDialog(request, false)}
        />
      );
    }

    return (
      <TextPromptModal
        key={request.id}
        title={title}
        description={request.message}
        label={request.options.label ?? t(props.locale, 'dialogPromptLabel')}
        initialValue={request.options.initialValue}
        placeholder={request.options.placeholder}
        confirmLabel={request.options.confirmLabel ?? t(props.locale, 'dialogConfirm')}
        cancelLabel={request.options.cancelLabel ?? t(props.locale, 'dialogCancel')}
        requiredMessage={request.options.required
          ? (request.options.requiredMessage ?? t(props.locale, 'dialogRequired'))
          : undefined}
        closeLabel={closeLabel}
        closeOnBackdrop={closeOnBackdrop(request)}
        onConfirm={(value) => completeDialog(request, value)}
        onClose={() => completeDialog(request, null)}
      />
    );
  };
});

export default DialogHost;
</script>
