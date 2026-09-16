<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { FormField } from './FormField.vue';
import { IconClose } from '../icons/index.ts';
import { fieldIds, fileListToArray, formatFileSize, removeFileAt } from './controls.ts';

type FileUploadProps = {
  value: File[];
  onValueChange: (v: File[]) => void;
  label?: string;
  hint?: string;
  error?: string;
  disabled?: boolean;
  required?: boolean;
  id?: string;
  name?: string;
  accept?: string;
  multiple?: boolean;
  maxFiles?: number;
};

let fileFallback = 0;

export const FileUpload = defineVueComponent<FileUploadProps>(
  ['value', 'onValueChange', 'label', 'hint', 'error', 'disabled', 'required', 'id', 'name', 'accept', 'multiple', 'maxFiles'],
  (props, context) => {
  const innerRef = ref<HTMLInputElement | null>(null);
  const dragging = ref(false);
  const mergeFiles = (incoming: File[]): void => {
    const multiple = props.multiple !== false;
    const base = multiple ? [...props.value] : [];
    const seen = new Set(base.map((f) => `${f.name}:${f.size}:${f.lastModified}`));
    for (const file of incoming) {
      const key = `${file.name}:${file.size}:${file.lastModified}`;
      if (seen.has(key)) continue;
      seen.add(key);
      base.push(file);
      if (props.maxFiles !== undefined && base.length >= props.maxFiles) break;
    }
    props.onValueChange(base);
  };
  context.expose({
    getValue: () => [...props.value],
    setValue: (v: File[]) => props.onValueChange([...v]),
    clear: () => {
      if (innerRef.value) innerRef.value.value = '';
      props.onValueChange([]);
    },
    focus: () => innerRef.value?.focus(),
  });
  return () => {
    fileFallback += 1;
    const { id, describedBy } = fieldIds(props, `tt-file-${fileFallback}`);
    const hintId = props.hint ? `${id}-hint` : undefined;
    const errorId = props.error ? `${id}-error` : undefined;
    const control = (
      <div class={`ui-file ${dragging.value ? 'is-dragging' : ''} ${props.error ? 'has-error' : ''} ${props.disabled ? 'is-disabled' : ''}`}>
        <button
          type="button"
          class="ui-file__dropzone"
          disabled={props.disabled}
          aria-describedby={describedBy([hintId, errorId])}
          aria-label={props.label ?? 'Upload files'}
          onClick={() => innerRef.value?.click()}
          onKeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              innerRef.value?.click();
            }
          }}
          onDragover={(e) => {
            if (props.disabled) return;
            e.preventDefault();
            dragging.value = true;
          }}
          onDragleave={() => { dragging.value = false; }}
          onDrop={(e) => {
            if (props.disabled) return;
            e.preventDefault();
            dragging.value = false;
            mergeFiles(fileListToArray(e.dataTransfer?.files));
          }}
        >
          <span class="ui-file__icon" aria-hidden>⇪</span>
          <span class="ui-file__text">Drop files here or <strong>browse</strong></span>
          {props.accept ? <span class="ui-file__accept">{props.accept}</span> : null}
        </button>
        <input
          ref={innerRef}
          id={id}
          name={props.name}
          type="file"
          accept={props.accept}
          multiple={props.multiple !== false ? true : undefined}
          disabled={props.disabled}
          required={props.required && props.value.length === 0}
          aria-invalid={Boolean(props.error)}
          style={{ position: 'absolute', width: 1, height: 1, opacity: 0, pointerEvents: 'none' }}
          tabindex={-1}
          onChange={(e) => mergeFiles(fileListToArray((e.currentTarget as HTMLInputElement).files))}
        />
        {props.value.length > 0 ? (
          <ul class="ui-file__list">
            {props.value.map((file, index) => (
              <li key={`${file.name}-${index}`} class="ui-file__item">
                <span class="ui-file__name">{file.name}</span>
                <span class="ui-file__size">{formatFileSize(file.size)}</span>
                <button
                  type="button"
                  class="ui-file__remove"
                  aria-label={`Remove ${file.name}`}
                  disabled={props.disabled}
                  onClick={() => {
                    props.onValueChange(removeFileAt(props.value, index));
                    if (innerRef.value && props.value.length === 1) innerRef.value.value = '';
                  }}
                >
                  <IconClose size={10} />
                </button>
              </li>
            ))}
          </ul>
        ) : null}
        {props.value.length > 0 ? (
          <button
            type="button"
            class="ui-file__clear"
            disabled={props.disabled}
            onClick={() => {
              if (innerRef.value) innerRef.value.value = '';
              props.onValueChange([]);
            }}
          >
            Clear all
          </button>
        ) : null}
      </div>
    );
    if (!props.label && !props.hint && !props.error) return control;
    return (
      <FormField label={props.label} hint={props.hint} error={props.error} htmlFor={id} required={props.required}>
        {control}
      </FormField>
    );
  };
  },
);

export default FileUpload;
</script>
