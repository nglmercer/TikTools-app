import type { JsonValue } from '../api/sync-adapter.ts';

const PREFIX = 'tiktools.sync.snapshot.';

/**
 * The last-known version of every element of one section, kept in this browser. Sections whose
 * records are not carried by a Cloud collection yet compare against it instead of Cloud's base.
 */
export type SectionSnapshot = Record<string, JsonValue>;

export function readSnapshot(scope: string): SectionSnapshot | null {
  try {
    const raw = localStorage.getItem(PREFIX + scope);
    if (raw === null) return null;
    const parsed: unknown = JSON.parse(raw);
    return parsed !== null && typeof parsed === 'object' && !Array.isArray(parsed) ? (parsed as SectionSnapshot) : null;
  } catch {
    return null;
  }
}

export function writeSnapshot(scope: string, snapshot: SectionSnapshot): void {
  try {
    localStorage.setItem(PREFIX + scope, JSON.stringify(snapshot));
  } catch {
    // Storage can be blocked or full; the section then starts a fresh snapshot next time.
  }
}
