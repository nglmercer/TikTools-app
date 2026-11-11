/**
 * Client-side journal for applying a Cloud package (`applyMode: clientJournal`).
 *
 * The Cloud never applies a package. The host applies each item through its own
 * install and import flows and records every step here. The journal is persisted
 * so that an interrupted apply can be recovered: an item that was running when the
 * app stopped is marked `interrupted` and is never retried blindly, because the
 * action may already have happened.
 */

export type EntryStatus = 'pending' | 'running' | 'done' | 'failed' | 'skipped' | 'interrupted';

export interface JournalEntry {
  readonly kind: string;
  readonly id: string;
  readonly version: string | null;
  readonly revision: number | null;
  readonly status: EntryStatus;
  readonly message: string | null;
}

export interface ApplyJournal {
  readonly packageId: string;
  readonly startedAt: number;
  readonly entries: readonly JournalEntry[];
}

export interface PreflightEntry {
  readonly kind: string;
  readonly id: string;
  readonly version: string | null;
  readonly revision: number | null;
  readonly status: string;
}

const KINDS = new Set(['plugin', 'template', 'profile', 'overlay']);
const STATUSES = new Set<EntryStatus>(['pending', 'running', 'done', 'failed', 'skipped', 'interrupted']);
const MAX_ENTRIES = 256;
const MAX_MESSAGE = 240;

/**
 * Starts a journal from a preflight. Refuses when any item is not ready: a package
 * with a missing or changed item must be checked again before anything is applied.
 */
export function createJournal(packageId: string, items: readonly PreflightEntry[], now: number): ApplyJournal {
  if (items.length === 0 || items.length > MAX_ENTRIES) throw new Error('package has no items to apply');
  const blocked = items.find((item) => item.status !== 'ready');
  if (blocked) throw new Error(`item ${blocked.kind}:${blocked.id} is ${blocked.status}`);
  return {
    packageId,
    startedAt: now,
    entries: items.map((item) => ({
      kind: item.kind,
      id: item.id,
      version: item.version,
      revision: item.revision,
      status: 'pending',
      message: null,
    })),
  };
}

/** Returns a journal with one entry changed. The input is never mutated. */
export function setEntry(journal: ApplyJournal, index: number, status: EntryStatus, message: string | null = null): ApplyJournal {
  if (index < 0 || index >= journal.entries.length) throw new Error('journal entry does not exist');
  return {
    ...journal,
    entries: journal.entries.map((entry, position) =>
      position === index ? { ...entry, status, message: message ? message.slice(0, MAX_MESSAGE) : null } : entry,
    ),
  };
}

/** Index of the first entry still waiting, or -1 when none is left. */
export function nextPending(journal: ApplyJournal): number {
  return journal.entries.findIndex((entry) => entry.status === 'pending');
}

/**
 * After a restart, anything that was running is interrupted. Those items are not
 * retried automatically; the user checks the installed list or library first.
 */
export function recoverInterrupted(journal: ApplyJournal): ApplyJournal {
  return {
    ...journal,
    entries: journal.entries.map((entry) =>
      entry.status === 'running'
        ? { ...entry, status: 'interrupted', message: 'The app stopped during this step. Check the result before retrying.' }
        : entry,
    ),
  };
}

export function summarize(journal: ApplyJournal): Record<EntryStatus, number> {
  const counts: Record<EntryStatus, number> = { pending: 0, running: 0, done: 0, failed: 0, skipped: 0, interrupted: 0 };
  for (const entry of journal.entries) counts[entry.status] += 1;
  return counts;
}

export function isFinished(journal: ApplyJournal): boolean {
  return journal.entries.every((entry) => entry.status !== 'pending' && entry.status !== 'running');
}

/** Parses a persisted journal. Anything malformed is refused rather than partly trusted. */
export function parseJournal(text: string): ApplyJournal | null {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return null;
  }
  if (value === null || typeof value !== 'object') return null;
  const record = value as Record<string, unknown>;
  if (typeof record.packageId !== 'string' || typeof record.startedAt !== 'number' || !Array.isArray(record.entries)) {
    return null;
  }
  if (record.entries.length > MAX_ENTRIES) return null;
  const entries: JournalEntry[] = [];
  for (const raw of record.entries) {
    if (raw === null || typeof raw !== 'object') return null;
    const entry = raw as Record<string, unknown>;
    if (typeof entry.kind !== 'string' || !KINDS.has(entry.kind)) return null;
    if (typeof entry.id !== 'string') return null;
    if (typeof entry.status !== 'string' || !STATUSES.has(entry.status as EntryStatus)) return null;
    entries.push({
      kind: entry.kind,
      id: entry.id,
      version: typeof entry.version === 'string' ? entry.version : null,
      revision: typeof entry.revision === 'number' ? entry.revision : null,
      status: entry.status as EntryStatus,
      message: typeof entry.message === 'string' ? entry.message.slice(0, MAX_MESSAGE) : null,
    });
  }
  return { packageId: record.packageId, startedAt: record.startedAt, entries };
}

export function serializeJournal(journal: ApplyJournal): string {
  return JSON.stringify(journal);
}
