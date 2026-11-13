import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCloudContext } from '@/entities/cloud-account/model/cloud-context.ts';
import { jsonEqual } from '@/shared/ui/json/json-diff.ts';
import { createOverlay, readOverlay, reviseOverlay, type OverlayRecord } from '../api/cloud-overlays-api.ts';
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

export interface OverlayComparison {
  id: string;
  local: OverlayDocument;
  base: OverlayDocument | null;
  cloud: OverlayDocument | null;
  conflict: boolean;
  canSend: boolean;
}

const copy = <T,>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
const same = jsonEqual;

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
  const loading = ref(false);
  const error = ref<string | null>(null);
  const copied = ref(false);
  const problems = computed(() => validateOverlay(draft.value));
  const base = ref<OverlayRecord | null>(null);
  const cached = ref<OverlayRecord | null>(null);
  const identity = ref(0);
  let reviewSequence = 0;
  let lastReview: { key: string; comparison: OverlayComparison } | null = null;
  const cloud = useCloudContext(() => {
    // A new local document has no account-owned data. Loaded/saved documents do.
    if (overlayId.value !== null) startNew('gift', 'New overlay');
    else { identity.value += 1; saving.value = false; loading.value = false; lastReview = null; }
  });

  const comparisonSource = computed(() => JSON.stringify([cloud.stamp(), identity.value, overlayId.value, toWire(draft.value), base.value, cached.value]));

  function startNew(kind: OverlayKind, title: string): void {
    identity.value += 1;
    draft.value = emptyOverlay(kind, title);
    overlayId.value = null;
    revision.value = null;
    error.value = null;
    base.value = null;
    cached.value = null;
    lastReview = null;
    saving.value = false;
    loading.value = false;
    copied.value = false;
  }

  async function load(id: string): Promise<boolean> {
    if (saving.value || loading.value) return false;
    const mine = identity.value;
    const context = cloud.stamp();
    const original = copy(draft.value);
    loading.value = true;
    try {
      const record = await readOverlay(client, id);
      if (mine !== identity.value || !cloud.isCurrent(context)) return false;
      if (!same(draft.value, original)) throw new Error('overlay-draft-changed');
      identity.value += 1;
      draft.value = copy(record.document);
      overlayId.value = record.id;
      revision.value = record.revision;
      base.value = copy(record);
      cached.value = copy(record);
      lastReview = null;
      error.value = null;
      loading.value = false;
      return true;
    } catch (failure) {
      if (mine === identity.value && cloud.isCurrent(context)) error.value = errorMessage(failure);
      return false;
    } finally {
      if (mine === identity.value && cloud.isCurrent(context)) loading.value = false;
    }
  }

  async function save(): Promise<boolean> {
    if (problems.value.length > 0 || saving.value || loading.value) return false;
    if (base.value?.revision !== cached.value?.revision) { error.value = 'overlay-cloud-changed'; return false; }
    const mine = identity.value;
    const context = cloud.stamp();
    saving.value = true;
    error.value = null;
    try {
      const document = copy(toWire(draft.value));
      const written = overlayId.value && revision.value !== null
        ? await reviseOverlay(client, overlayId.value, document, revision.value)
        : await createOverlay(client, document);
      if (mine !== identity.value || !cloud.isCurrent(context)) return false;
      overlayId.value = written.id;
      revision.value = written.revision;
      // A save acknowledges the submitted snapshot, not newer edits made while awaiting it.
      base.value = { ...written, document };
      cached.value = copy(base.value);
      lastReview = null;
      return true;
    } catch (failure) {
      if (mine === identity.value && cloud.isCurrent(context)) error.value = errorMessage(failure);
      return false;
    } finally {
      if (mine === identity.value && cloud.isCurrent(context)) saving.value = false;
    }
  }

  /** Download only the latest revision into the comparison cache; never replace a draft. */
  async function fetchRevision(): Promise<boolean> {
    if (!overlayId.value) { error.value = 'overlay-not-saved'; return false; }
    if (saving.value || loading.value) return false;
    const id = overlayId.value;
    const mine = identity.value;
    const context = cloud.stamp();
    loading.value = true;
    error.value = null;
    try {
      const record = await readOverlay(client, id);
      if (mine !== identity.value || !cloud.isCurrent(context)) return false;
      if (revision.value !== null && record.revision < Math.max(revision.value, cached.value?.revision ?? 0)) throw new Error('overlay-review-changed');
      cached.value = copy(record);
      return true;
    } catch (failure) {
      if (mine === identity.value && cloud.isCurrent(context)) error.value = errorMessage(failure);
      return false;
    } finally {
      if (mine === identity.value && cloud.isCurrent(context)) loading.value = false;
    }
  }

  /** Stable IDs bind actions to both documents, their revisions and this account context. */
  function review(): OverlayComparison | null {
    const local = copy(toWire(draft.value));
    const remote = cached.value?.document ?? null;
    if (remote !== null && base.value?.revision === cached.value?.revision && same(local, toWire(remote))) { lastReview = null; return null; }
    const key = comparisonSource.value;
    if (lastReview?.key === key) return copy(lastReview.comparison);
    const localChanged = base.value === null || !same(local, toWire(base.value.document));
    const remoteChanged = base.value?.revision !== cached.value?.revision;
    const comparison: OverlayComparison = {
      id: `overlay-review-${++reviewSequence}`,
      local,
      base: base.value ? copy(toWire(base.value.document)) : null,
      cloud: remote ? copy(toWire(remote)) : null,
      conflict: localChanged && remoteChanged,
      canSend: localChanged && !remoteChanged && problems.value.length === 0,
    };
    lastReview = { key, comparison };
    return copy(comparison);
  }

  function requireReview(id: string): OverlayComparison {
    const current = review();
    if (!current || current.id !== id || saving.value || loading.value) throw new Error('overlay-review-changed');
    return current;
  }

  async function saveReview(id: string): Promise<boolean> {
    if (!requireReview(id).canSend) throw new Error('overlay-cloud-changed');
    return save();
  }

  /** Local adoption of the reviewed cache; no network request or new Cloud revision. */
  function applyReview(id: string): void {
    const current = requireReview(id);
    if (!current.cloud || !cached.value) throw new Error('overlay-not-saved');
    draft.value = copy(current.cloud);
    base.value = copy(cached.value);
    revision.value = cached.value.revision;
    error.value = null;
    copied.value = false;
    lastReview = null;
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
    loading,
    comparisonSource,
    error,
    copied,
    problems,
    copyObsUrl,
    startNew,
    load,
    save,
    fetchRevision,
    review,
    saveReview,
    applyReview,
    setCanvas,
    setTitle,
    setKind,
    add,
    patch,
    remove,
    replaceElement,
  };
});
