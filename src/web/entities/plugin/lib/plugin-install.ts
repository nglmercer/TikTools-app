import type { MediaPickerOptions } from '../../../../shared/messages.ts';

export function pluginPickerOptions(title: string): MediaPickerOptions {
  return { mode: 'file', kind: 'other', title, extensions: ['plugin'] };
}
