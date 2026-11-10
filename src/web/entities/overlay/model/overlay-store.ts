import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { createOverlay, readOverlay, reviseOverlay } from '../api/overlays-api.ts';
import {
  addElement,
  emptyOverlay,
  removeElement,
  toWire,
  updateElement,
  validateOverlay,
  type OverlayDocument,
  type OverlayElement,
  type OverlayKind,
} from '../lib/overlay-schema.ts';

/**
 * One overlay being edited. The draft lives in memory until saved. A save is refused
 * while the schema reports problems; a saved overlay is revised with the revision the
 * editor last read, so a concurrent change is reported, not overwritten.
 */
export const useOverlayStore = defineStore('overlay', () => {
  const client = useControlClient();
  const draft = ref<OverlayDocument>(emptyOverlay('gift', 'New overlay'));
  const overlayId = ref<string | null>(null);
  const revision = ref<number | null>(null);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const problems = computed(() => validateOverlay(draft.value));

  function startNew(kind: OverlayKind, title: string): void {
    draft.value = emptyOverlay(kind, title);
    overlayId.value = null;
    revision.value = null;
    error.value = null;
  }

  async function load(id: string): Promise<boolean> {
    try {
      const record = await readOverlay(client, id);
      draft.value = record.document;
      overlayId.value = record.id;
      revision.value = record.revision;
      error.value = null;
      return true;
    } catch (failure) {
      error.value = errorMessage(failure);
      return false;
    }
  }

  async function save(): Promise<boolean> {
    if (problems.value.length > 0 || saving.value) return false;
    saving.value = true;
    error.value = null;
    try {
      const document = toWire(draft.value) as OverlayDocument;
      const written = overlayId.value && revision.value !== null
        ? await reviseOverlay(client, overlayId.value, document, revision.value)
        : await createOverlay(client, document);
      overlayId.value = written.id;
      revision.value = written.revision;
      return true;
    } catch (failure) {
      error.value = errorMessage(failure);
      return false;
    } finally {
      saving.value = false;
    }
  }

  function setCanvas(width: number, height: number): void {
    draft.value = { ...draft.value, canvas: { width, height } };
  }

  function setTitle(title: string): void {
    draft.value = { ...draft.value, title };
  }

  function setKind(kind: OverlayKind): void {
    draft.value = { ...draft.value, kind };
  }

  function add(type: OverlayElement['type']): void {
    draft.value = addElement(draft.value, type);
  }

  function patch(id: string, change: Partial<Pick<OverlayElement, 'x' | 'y' | 'width' | 'height' | 'opacity'>>): void {
    draft.value = updateElement(draft.value, id, change);
  }

  function remove(id: string): void {
    draft.value = removeElement(draft.value, id);
  }

  function replaceElement(id: string, next: OverlayElement): void {
    draft.value = { ...draft.value, elements: draft.value.elements.map((element) => (element.id === id ? next : element)) };
  }

  return {
    draft,
    overlayId,
    revision,
    saving,
    error,
    problems,
    startNew,
    load,
    save,
    setCanvas,
    setTitle,
    setKind,
    add,
    patch,
    remove,
    replaceElement,
  };
});
