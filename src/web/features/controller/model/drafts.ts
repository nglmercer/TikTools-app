import { computed, ref, type Ref } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import type { ControllerStatus } from './controller.ts';

export interface LocalDraft { text: string; revision: string }
interface StoredDraft { id: string; baseRevision: string; text: string; updatedAt: number }
interface Metadata { id: string; baseRevision: string; updatedAt: number; textBytes: number }
interface Page { epoch: string; viewRevision: string; storeRevision: string; rows: Metadata[]; next: number | null }
interface ReadReply { epoch: string; viewRevision: string; storeRevision: string; draft: StoredDraft | null }
interface WriteReply { epoch: string; viewRevision: string; storeRevision: string }
export interface DraftSaveTarget { epoch: string; viewRevision: string; scope: string; id: string; text: string; revision: string; expectedStoreRevision: string }
interface Context {
  status: Ref<ControllerStatus>; ready: Ref<boolean>; scope: Ref<string>;
  generation: () => number; stopped: () => boolean;
  read: (id: string) => LocalDraft | undefined;
  put: (id: string, draft: LocalDraft) => void; drop: (id: string) => void;
}
const key = (s: ControllerStatus) => `${s.epoch}:${s.viewRevision}`;
const bytes = (text: string) => new TextEncoder().encode(text).length;
const TEXT_BYTES = 256 * 1024;
const errorCode = (failure: unknown) => failure instanceof ControlCallError ? failure.code : 'draft_unavailable';

/** Explicit private native storage. The in-memory editor remains independent. */
export function createDraftPersistence(control: ControlClient, ctx: Context) {
  const error = ref(''); const busy = ref(false); const loading = ref(false);
  const metadata = ref<Metadata[]>([]); const metadataScope = ref(''); let loaded = '';
  const copies = new Map<string, Map<string, { revision: string; draft: StoredDraft | null; unconfirmed?: boolean }>>();
  const changed = ref(0); const pending = new Map<string, Promise<void>>();
  const supported = computed(() => ctx.status.value.draftsSupported === true && ctx.status.value.hello?.capabilities.includes('workflows.read') === true);
  function permitted(id: string): boolean {
    const ids = ctx.status.value.hello?.resourceRestrictions?.['workflow'];
    return supported.value && ctx.ready.value && (ids === undefined || ids.includes(id));
  }
  const visible = computed(() => supported.value && ctx.ready.value && metadataScope.value === ctx.scope.value ? metadata.value.filter((m) => permitted(m.id)) : []);
  function own() { let map = copies.get(ctx.scope.value); if (!map) { map = new Map(); copies.set(ctx.scope.value, map); } return map; }
  function saved(id: string): StoredDraft | null { void changed.value; const copy = copies.get(ctx.scope.value)?.get(id); return permitted(id) && !copy?.unconfirmed ? copy?.draft ?? null : null; }
  function known(id: string): boolean { void changed.value; return permitted(id) && copies.get(ctx.scope.value)?.has(id) === true; }
  function same(s: ControllerStatus, scope: string, generation: number): boolean {
    return !ctx.stopped() && generation === ctx.generation() && scope === ctx.scope.value && key(s) === key(ctx.status.value) && ctx.status.value.phase === 'ready';
  }
  async function refresh(s: ControllerStatus, generation: number): Promise<void> {
    if (!supported.value || s.phase !== 'ready') { metadata.value = []; metadataScope.value = ''; loaded = ''; return; }
    if (loaded === key(s) && metadataScope.value === ctx.scope.value) return;
    const scope = ctx.scope.value;
    try {
      const rows: Metadata[] = []; let offset: number | null = 0; let revision: string | undefined;
      const positions = new Set<number>();
      while (offset !== null) {
        if (positions.has(offset) || positions.size >= 4) throw new Error('Invalid draft pagination'); positions.add(offset);
        const page: Page = await control.call('controller.drafts.list', { epoch: s.epoch, viewRevision: s.viewRevision, offset });
        if (page.epoch !== s.epoch || page.viewRevision !== s.viewRevision || (revision !== undefined && revision !== page.storeRevision)) throw new Error('Saved drafts changed');
        revision = page.storeRevision; rows.push(...page.rows); offset = page.next;
      }
      const after = await control.call<ControllerStatus>('controller.status');
      if (!same(s, scope, generation) || key(after) !== key(s) || after.phase !== 'ready') return;
      if (rows.length > 128 || new Set(rows.map((r) => r.id)).size !== rows.length) throw new Error('Invalid draft list');
      metadata.value = rows; metadataScope.value = scope; loaded = key(s);
    } catch (failure) { if (same(s, scope, generation)) { error.value = errorCode(failure); loaded = ''; } }
  }
  async function load(id: string, reload = false): Promise<void> {
    if (!permitted(id) || busy.value) return;
    if (!reload && known(id)) return;
    const s = ctx.status.value; const scope = ctx.scope.value; const generation = ctx.generation(); const token = `${scope}:${s.epoch}:${generation}:${id}`;
    if (pending.has(token)) return pending.get(token);
    const request = (async () => {
      loading.value = true; error.value = '';
      try {
        const result = await control.call<ReadReply>('controller.drafts.get', { epoch: s.epoch, viewRevision: s.viewRevision, id });
        const after = await control.call<ControllerStatus>('controller.status');
        if (!same(s, scope, generation) || key(after) !== key(s) || after.phase !== 'ready') return;
        if (result.epoch !== s.epoch || result.viewRevision !== s.viewRevision || (result.draft && (result.draft.id !== id || bytes(result.draft.text) > TEXT_BYTES))) throw new Error('Invalid saved draft');
        own().set(id, { revision: result.storeRevision, draft: result.draft }); changed.value += 1;
        // Never replace a newer unsaved edit when a file read returns late.
        if (result.draft && !ctx.read(id)) ctx.put(id, { text: result.draft.text, revision: result.draft.baseRevision });
      } catch (failure) { if (same(s, scope, generation)) error.value = errorCode(failure); }
      finally { pending.delete(token); if (generation === ctx.generation() && scope === ctx.scope.value) loading.value = false; }
    })(); pending.set(token, request); return request;
  }
  function prepare(id: string): DraftSaveTarget | null {
    void changed.value;
    const s = ctx.status.value; const draft = ctx.read(id); const copy = own().get(id);
    if (!permitted(id) || busy.value || loading.value || !draft || !copy || copy.unconfirmed || bytes(draft.text) > TEXT_BYTES) return null;
    return { epoch: s.epoch, viewRevision: s.viewRevision, scope: ctx.scope.value, id, text: draft.text, revision: draft.revision, expectedStoreRevision: copy.revision };
  }
  function current(t: DraftSaveTarget): boolean {
    const s = ctx.status.value; const draft = ctx.read(t.id); const copy = copies.get(t.scope)?.get(t.id);
    return permitted(t.id) && !busy.value && !loading.value && t.epoch === s.epoch && t.viewRevision === s.viewRevision && t.scope === ctx.scope.value &&
      draft?.text === t.text && draft?.revision === t.revision && copy?.revision === t.expectedStoreRevision && !copy.unconfirmed;
  }
  function advance(before: string, after: string) { for (const copy of own().values()) if (copy.revision === before) copy.revision = after; changed.value += 1; loaded = ''; }
  async function save(t: DraftSaveTarget): Promise<boolean> {
    if (!current(t)) return false;
    const s = ctx.status.value; const generation = ctx.generation(); busy.value = true; error.value = '';
    try {
      const result = await control.call<WriteReply>('controller.drafts.save', { epoch: t.epoch, viewRevision: t.viewRevision, id: t.id, expectedStoreRevision: t.expectedStoreRevision, baseRevision: t.revision, text: t.text, acknowledgeStorage: true });
      if (!same(s, t.scope, generation) || result.epoch !== t.epoch || result.viewRevision !== t.viewRevision) return false;
      advance(t.expectedStoreRevision, result.storeRevision);
      own().set(t.id, { revision: result.storeRevision, draft: { id: t.id, baseRevision: t.revision, text: t.text, updatedAt: Date.now() } }); changed.value += 1;
      await refresh(s, generation); return true;
    } catch (failure) {
      if (same(s, t.scope, generation)) { const copy = own().get(t.id); if (copy) copy.unconfirmed = true; changed.value += 1; error.value = errorCode(failure); }
      return false;
    }
    finally { if (generation === ctx.generation()) busy.value = false; }
  }
  async function remove(id: string, discardText: boolean): Promise<boolean> {
    if (!permitted(id) || busy.value) return false;
    const initial = ctx.status.value; const initialScope = ctx.scope.value; const initialGeneration = ctx.generation();
    let discard = ctx.read(id);
    await load(id);
    if (!same(initial, initialScope, initialGeneration)) return false;
    const copy = own().get(id); if (!copy || !permitted(id)) return false;
    if (!discard && copy.draft && ctx.read(id)?.text === copy.draft.text && ctx.read(id)?.revision === copy.draft.baseRevision) discard = ctx.read(id);
    const dropUnchanged = () => { if (discardText && ctx.read(id)?.text === discard?.text && ctx.read(id)?.revision === discard?.revision) ctx.drop(id); };
    if (copy.unconfirmed) { error.value = 'draft_review_required'; return false; }
    if (!copy.draft && !visible.value.some((m) => m.id === id)) { dropUnchanged(); return true; }
    const s = ctx.status.value; const scope = ctx.scope.value; const generation = ctx.generation(); busy.value = true; error.value = '';
    try {
      const result = await control.call<WriteReply>('controller.drafts.delete', { epoch: s.epoch, viewRevision: s.viewRevision, id, expectedStoreRevision: copy.revision });
      if (!same(s, scope, generation) || result.epoch !== s.epoch || result.viewRevision !== s.viewRevision) return false;
      advance(copy.revision, result.storeRevision); own().set(id, { revision: result.storeRevision, draft: null }); changed.value += 1;
      metadata.value = metadata.value.filter((m) => m.id !== id); dropUnchanged();
      await refresh(s, generation); return true;
    } catch (failure) {
      if (same(s, scope, generation)) { copy.unconfirmed = true; changed.value += 1; error.value = errorCode(failure); }
      return false;
    }
    finally { if (generation === ctx.generation()) busy.value = false; }
  }
  function reset(): void { busy.value = false; loading.value = false; metadata.value = []; metadataScope.value = ''; loaded = ''; error.value = ''; copies.clear(); changed.value += 1; }
  function stop(): void { reset(); copies.clear(); pending.clear(); }
  return { supported, visible, saved, known, error, busy, loading, refresh, load, prepare, current, save, remove, reset, stop };
}
