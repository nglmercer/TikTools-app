<script setup lang="ts">
import {onMounted,onUnmounted,ref} from 'vue';
import {mountEditor} from '../../universal/editor-ui.js';
import {fromTikTools} from '../../universal/index.js';
import {createWidgetRuntime} from '../shared/widget-runtime.ts';
defineProps<{locale:string}>();
const root=ref<HTMLDivElement>();
let editor:ReturnType<typeof mountEditor>|undefined;
let runtime:ReturnType<typeof createWidgetRuntime>|undefined;
onMounted(()=>{
 if(!root.value)return;
 editor=mountEditor(root.value,{locale:root.value.lang,subscribe(fn){
  runtime=createWidgetRuntime({onEnvelope(envelope){const event=fromTikTools(envelope);if(event)fn(event);}});
  runtime.start();return ()=>runtime?.stop();
 }});
});
onUnmounted(()=>editor?.dispose());
</script>
<template><div ref="root" :lang="locale" /></template>

<style>
/*
 * Host theme for the vendored portable-widget editor (src/universal), which
 * renders plain HTML. Kept here so the SDK itself stays host-agnostic.
 */
.universal-editor {
  padding: var(--space-5);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
}

.universal-editor h2 {
  margin: 0 0 var(--space-5);
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
}

.universal-editor > label,
.universal-editor .uw-toolbar label {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--space-2);
  margin: 0 0 var(--space-4);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.universal-editor .uw-toolbar {
  gap: var(--space-3);
  margin: 0 0 var(--space-5);
  padding: var(--space-5);
  border: var(--border-width) solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  background: var(--color-bg-panel);
}

.universal-editor button,
.universal-editor input[type='file']::file-selector-button {
  height: var(--control-height-md);
  padding: 0 var(--space-5);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-control);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  cursor: pointer;
}

.universal-editor button:hover:not(:disabled),
.universal-editor input[type='file']::file-selector-button:hover {
  border-color: var(--color-border-strong);
  background: var(--color-bg-control-hover);
}

.universal-editor button:disabled {
  border-color: var(--color-border-subtle);
  background: var(--color-bg-control-active);
  color: var(--color-text-disabled);
  cursor: default;
  opacity: 1;
}

.universal-editor input[type='file'] {
  max-width: 100%;
  color: var(--color-text-subtle);
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
}

.universal-editor input[type='file']::file-selector-button {
  margin-right: var(--space-4);
}

.universal-editor input[type='checkbox'] {
  width: 14px;
  height: 14px;
  accent-color: var(--color-accent);
}

.universal-editor input[type='number'],
.universal-editor input[type='text'],
.universal-editor textarea,
.universal-editor select {
  min-height: var(--control-height-md);
  padding: 0 var(--control-padding-x);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-field);
  color: var(--color-text);
  font-family: var(--font-family);
}

.universal-editor textarea {
  padding: var(--space-3) var(--control-padding-x);
  font-family: var(--font-family-mono);
}

.universal-editor [role='status'] {
  margin: 0 0 var(--space-5);
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
}

.universal-editor .uw-stage {
  border: var(--border-width) solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  background: var(--color-bg-app);
}
</style>

