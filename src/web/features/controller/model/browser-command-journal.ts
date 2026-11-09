/** Durable, origin-local command intents. No credentials or workflow bodies are stored. */
export type BrowserCommandBinding = { origin: string; hostId: string; principalId: string; sessionId: string };
export type BrowserCommandIntent = { commandId: string; operation: 'disable' | 'delete'; resourceId: string; hostInstanceId: string; expectedRevision: string; createdAt: number };
export type BrowserCommandResult = { intent: BrowserCommandIntent; outcome: 'succeeded' | 'rejected' | 'reviewed-unknown'; revision: string | null; errorCode: string | null; recordedAt: number };
export type BrowserCommandJournal = { binding: BrowserCommandBinding; pending: BrowserCommandIntent | null; recent: BrowserCommandResult[] };
type Stored = BrowserCommandJournal & { version: 1; key: string };
const DB = 'tiktools-remote-command-journal-v1';
const STORE = 'journals';
const id = (v: unknown): v is string => typeof v === 'string' && v.length > 0 && new TextEncoder().encode(v).length <= 256 && !/\p{Cc}/u.test(v);
const uuid = (v: unknown): v is string => typeof v === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(v);
const revision = (v: unknown): v is string => typeof v === 'string' && /^(0|[1-9][0-9]{0,18})$/.test(v) && BigInt(v) <= 9223372036854775807n;
const time = (v: unknown): v is number => typeof v === 'number' && Number.isSafeInteger(v) && v >= 0;
const object = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
const keys = (v: Record<string, unknown>, expected: string[]): boolean => Object.keys(v).length === expected.length && expected.every((k) => Object.hasOwn(v, k));
function validBinding(v: unknown): v is BrowserCommandBinding {
  if (!object(v) || !keys(v, ['origin', 'hostId', 'principalId', 'sessionId']) || !id(v['origin']) || !id(v['hostId']) || !id(v['principalId']) || !id(v['sessionId'])) return false;
  try { const url = new URL(v['origin']); return url.origin === v['origin'] && url.protocol === 'https:' && !url.username && !url.password; } catch { return false; }
}
function validIntent(v: unknown): v is BrowserCommandIntent {
  return object(v) && keys(v, ['commandId', 'operation', 'resourceId', 'hostInstanceId', 'expectedRevision', 'createdAt']) &&
    uuid(v['commandId']) && (v['operation'] === 'disable' || v['operation'] === 'delete') && id(v['resourceId']) && id(v['hostInstanceId']) && revision(v['expectedRevision']) && time(v['createdAt']);
}
function validResult(v: unknown): v is BrowserCommandResult {
  return object(v) && keys(v, ['intent', 'outcome', 'revision', 'errorCode', 'recordedAt']) && validIntent(v['intent']) &&
    (v['outcome'] === 'succeeded' || v['outcome'] === 'rejected' || v['outcome'] === 'reviewed-unknown') &&
    (v['revision'] === null || revision(v['revision'])) && (v['errorCode'] === null || id(v['errorCode'])) && time(v['recordedAt']) &&
    (v['outcome'] === 'succeeded' ? v['revision'] !== null && v['errorCode'] === null : v['revision'] === null) &&
    (v['outcome'] !== 'reviewed-unknown' || v['errorCode'] === null);
}
const keyFor = (b: BrowserCommandBinding): string => JSON.stringify([b.origin, b.hostId, b.principalId, b.sessionId]);
function decode(v: unknown, binding: BrowserCommandBinding): Stored {
  const key = keyFor(binding);
  if (v === undefined) return { version: 1, key, binding: { ...binding }, pending: null, recent: [] };
  if (!object(v) || !keys(v, ['version', 'key', 'binding', 'pending', 'recent']) || v['version'] !== 1 || v['key'] !== key ||
      !validBinding(v['binding']) || keyFor(v['binding']) !== key || !(v['pending'] === null || validIntent(v['pending'])) ||
      !Array.isArray(v['recent']) || v['recent'].length > 16 || !v['recent'].every(validResult)) throw new Error('command journal unavailable');
  const commands = [...v['recent'].map((item: BrowserCommandResult) => item.intent.commandId), ...(v['pending'] ? [(v['pending'] as BrowserCommandIntent).commandId] : [])];
  if (new Set(commands).size !== commands.length) throw new Error('command journal unavailable');
  return v as Stored;
}
function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    let abandoned = false;
    const request = indexedDB.open(DB, 1);
    request.onupgradeneeded = () => { request.result.createObjectStore(STORE, { keyPath: 'key' }); };
    request.onerror = () => reject(new Error('command journal unavailable'));
    request.onblocked = () => { abandoned = true; reject(new Error('command journal unavailable')); };
    request.onsuccess = () => { if (abandoned) request.result.close(); else resolve(request.result); };
  });
}

/** Each operation uses one strict read-write transaction, so two tabs cannot
 * both create a pending intent. The transaction must commit before dispatch. */
async function access<T>(binding: BrowserCommandBinding, allowCreate: boolean, change: (journal: BrowserCommandJournal) => T): Promise<T> {
  if (!validBinding(binding)) throw new Error('invalid command binding');
  const stable = { ...binding }; const database = await open();
  return new Promise((resolve, reject) => {
    let result: T; let failure: unknown;
    let transaction: IDBTransaction;
    try { transaction = database.transaction(STORE, 'readwrite', { durability: 'strict' }); }
    catch (error) { database.close(); reject(error); return; }
    const store = transaction.objectStore(STORE);
    const request = store.get(keyFor(stable));
    request.onsuccess = () => {
      try {
        if (request.result === undefined && !allowCreate) throw new Error('command journal missing; review storage loss');
        const record = decode(request.result, stable);
        result = change(record);
        if (object(result) && typeof result['then'] === 'function') throw new Error('command journal callback must be synchronous');
        decode(record, stable);
        store.put(record);
      } catch (error) { failure = error; transaction.abort(); }
    };
    transaction.oncomplete = () => { database.close(); resolve(result!); };
    transaction.onabort = () => { database.close(); reject(failure ?? new Error('command journal unavailable')); };
    transaction.onerror = () => { /* abort handles the failure */ };
  });
}
export function withBrowserCommandJournal<T>(binding: BrowserCommandBinding, change: (journal: BrowserCommandJournal) => T): Promise<T> {
  return access(binding, false, change);
}
/** Missing storage may contain a lost command. Explicit review is mandatory
 * even for a new session, which cannot be distinguished from storage loss. */
export function initializeBrowserCommandJournal(binding: BrowserCommandBinding, acknowledgeStorageLoss: boolean): Promise<void> {
  if (acknowledgeStorageLoss !== true) return Promise.reject(new Error('storage loss review required'));
  return access(binding, true, () => undefined);
}
export function beginBrowserCommand(journal: BrowserCommandJournal, intent: BrowserCommandIntent): void {
  if (!validIntent(intent)) throw new Error('invalid command intent');
  if (journal.pending || journal.recent.some((item) => item.intent.commandId === intent.commandId)) throw new Error('command already pending');
  journal.pending = structuredClone(intent);
}
export function finishBrowserCommand(journal: BrowserCommandJournal, commandId: string, result: Omit<BrowserCommandResult, 'intent'>): void {
  if (!journal.pending || journal.pending.commandId !== commandId) throw new Error('pending command changed');
  const entry = { ...result, intent: structuredClone(journal.pending) };
  if (!validResult(entry)) throw new Error('invalid command result');
  journal.recent = [entry, ...journal.recent].slice(0, 16); journal.pending = null;
}
