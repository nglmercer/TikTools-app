import { reactive } from 'vue';

export type DialogKind = 'alert' | 'confirm' | 'prompt';

type DialogOptions = {
  title?: string;
  description?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  okLabel?: string;
  closeLabel?: string;
  label?: string;
  initialValue?: string;
  placeholder?: string;
  required?: boolean;
  requiredMessage?: string;
  danger?: boolean;
  closeOnBackdrop?: boolean;
};

export type AlertOptions = Omit<DialogOptions, 'cancelLabel' | 'confirmLabel' | 'danger' | 'initialValue' | 'label' | 'placeholder' | 'required' | 'requiredMessage'>;
export type ConfirmOptions = Omit<DialogOptions, 'initialValue' | 'label' | 'placeholder' | 'required' | 'requiredMessage'>;
export type PromptOptions = Omit<DialogOptions, 'danger' | 'okLabel'>;

export type AlertDialogRequest = {
  id: number;
  kind: 'alert';
  message: string;
  options: AlertOptions;
  resolve: (value: void) => void;
};

export type ConfirmDialogRequest = {
  id: number;
  kind: 'confirm';
  message: string;
  options: ConfirmOptions;
  resolve: (value: boolean) => void;
};

export type PromptDialogRequest = {
  id: number;
  kind: 'prompt';
  message: string;
  options: PromptOptions;
  resolve: (value: string | null) => void;
};

export type DialogRequest = AlertDialogRequest | ConfirmDialogRequest | PromptDialogRequest;

const dialogQueue = reactive<DialogRequest[]>([]);
let nextDialogId = 1;

function enqueue<T extends DialogRequest>(request: Omit<T, 'id'>): Promise<T extends AlertDialogRequest ? void : T extends ConfirmDialogRequest ? boolean : string | null> {
  return new Promise((resolve) => {
    dialogQueue.push({ ...request, id: nextDialogId++, resolve } as T);
  }) as Promise<T extends AlertDialogRequest ? void : T extends ConfirmDialogRequest ? boolean : string | null>;
}

export function completeDialog(request: DialogRequest, value: void | boolean | string | null): void {
  const index = dialogQueue.indexOf(request);
  if (index < 0) return;

  dialogQueue.splice(index, 1);
  if (request.kind === 'alert') request.resolve(undefined);
  else if (request.kind === 'confirm') request.resolve(Boolean(value));
  else request.resolve(typeof value === 'string' ? value : null);
}

export function useDialogs() {
  const alert = (message: string, options: AlertOptions = {}): Promise<void> =>
    enqueue<AlertDialogRequest>({ kind: 'alert', message, options, resolve: () => undefined });

  const confirm = (message: string, options: ConfirmOptions = {}): Promise<boolean> =>
    enqueue<ConfirmDialogRequest>({ kind: 'confirm', message, options, resolve: () => false });

  const prompt = (message: string, options: PromptOptions = {}): Promise<string | null> =>
    enqueue<PromptDialogRequest>({ kind: 'prompt', message, options, resolve: () => null });

  return { alert, confirm, prompt };
}

export { dialogQueue };
