import type {
  MediaPickerOptions,
  MediaSelection,
  MediaSelectionHandler,
} from '../../shared/messages.ts';
import type { ControlClient } from '../platform/control-client.ts';
import { ControlCallError, errorMessage } from '../platform/control-client.ts';

export interface MediaPickResult {
  selection: MediaSelection | null;
}

/** Native file dialogs through the desktop-only media.pick method. */
export function useMedia(control: ControlClient) {
  const openMediaPicker = (
    options: MediaPickerOptions,
    onSelected: MediaSelectionHandler,
  ): void => {
    void (async () => {
      try {
        const result = await control.call<MediaPickResult>('media.pick', {
          ...(options.mode ? { mode: options.mode } : {}),
          ...(options.kind ? { kind: options.kind } : {}),
          ...(options.title ? { title: options.title } : {}),
          ...(options.initialDirectory ? { initialDirectory: options.initialDirectory } : {}),
          ...(options.extensions?.length
            ? { extensions: options.extensions.slice(0, 32) }
            : {}),
        });
        onSelected(result.selection ?? null);
      } catch (failure) {
        if (
          failure instanceof ControlCallError &&
          (failure.code === 'capability_unavailable' || failure.code === 'transport')
        ) {
          onSelected(null, 'Native media picker is unavailable in this preview.');
          return;
        }
        onSelected(null, errorMessage(failure));
      }
    })();
  };

  return {
    openMediaPicker,
  };
}
