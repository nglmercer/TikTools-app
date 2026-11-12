import { computed, markRaw, ref } from 'vue';
import type { JsonValue } from '../../../../../automation/types.ts';
import type { MediaSelection } from '../../../../../shared/messages.ts';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import { textValue } from './schema-fields.ts';
import type { SchemaFieldControlProps } from './schema-control-props.ts';

const AUDIO_EXTENSIONS = ['aac', 'flac', 'm4a', 'mp3', 'mp4', 'oga', 'ogg', 'opus', 'wav', 'webm'];

/** Field-lifetime picker/menu state survives switching the rendered control kind. */
export function useSchemaFieldInput(props: Readonly<SchemaFieldControlProps>, update: (value: JsonValue) => void) {
  const pickerError = ref('');
  const variablesAnchor = ref<HTMLElement | null>(null);
  const variablesOpen = ref(false);
  const text = computed(() => textValue(props.value));
  const variableItems = computed<MenuEntry[]>(() =>
    (props.suggestions ?? []).map((suggestion) => ({
      id: suggestion.value,
      label: suggestion.label,
      shortcut: suggestion.detail,
      icon: markRaw(IconFlow),
    })),
  );
  const showVariables = computed(() => props.field.template && variableItems.value.length > 0);

  function insertVariable(action: MenuAction): void {
    update(`${text.value}{{ ${action.id} }}`);
  }

  function pickMedia(): void {
    const media = props.field.media;
    if (!media || !props.openMediaPicker) return;
    pickerError.value = '';
    props.openMediaPicker(
      {
        mode: media.mode,
        kind: media.kind,
        extensions: media.extensions ?? (media.kind === 'audio' ? AUDIO_EXTENSIONS : undefined),
        title: props.field.label,
      },
      (selection: MediaSelection | null, error?: string) => {
        if (error) pickerError.value = error;
        else if (selection?.type === 'file') update(selection.file as unknown as JsonValue);
        else if (selection?.type === 'directory') update(selection.directory as unknown as JsonValue);
      },
    );
  }
  function updateMediaPath(path: string): void { pickerError.value = ''; update(path); }
  return { pickerError, variablesAnchor, variablesOpen, text, variableItems, showVariables, insertVariable, pickMedia, updateMediaPath };
}
