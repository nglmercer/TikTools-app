import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { createOverlay, readOverlay, reviseOverlay } from '../api/cloud-overlays-api.ts';
import { encodeOverlayFragment } from '../../../../shared/cloud-overlay-fragment.ts';
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
} from '../../../../shared/cloud-overlay-schema.ts';

/**
 * One overlay being edited. The draft lives in memory until saved. A save is refused
 * while the schema reports problems; a saved overlay is revised with the revision the
 * editor last read, so a concurrent change is reported, not overwritten.
 */
export const useCloudOverlayStore = defineStore('overlay', () => {
  const client = useControlClient();
  const draft = ref<OverlayDocument>(emptyOverlay('gift', 'New overlay'));
  const overlayId = ref<string | null>(null);
  const revision = ref<number | null>(null);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const copied = ref(false);
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

  /**
   * Copies the OBS Browser Source URL of the draft. The document travels in the URL, so a
   * draft that does not fit is refused before anything is copied.
   */
  async function copyObsUrl(): Promise<boolean> {
    copied.value = false;
    if (problems.value.length > 0) return false;
    const payload = encodeOverlayFragment(toWire(draft.value));
    if (payload === null) {
      error.value = 'too-large';
      return false;
    }
    try {
      await client.call('widgets.copyCloudOverlayUrl', { overlay: payload });
      copied.value = true;
      error.value = null;
      return true;
    } catch (failure) {
      error.value = errorMessage(failure);
      return false;
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
    copied,
    problems,
    copyObsUrl,
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
