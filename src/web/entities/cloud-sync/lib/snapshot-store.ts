import { appStorage } from '@/shared/lib/app-storage.ts';
import { shallowRef } from 'vue';
import type { JsonValue } from '../api/sync-adapter.ts';

const PREFIX = 'tiktools.sync.snapshot.';

/**
 * The last-known version of every element of one section, kept in this browser. Sections whose
 * records are not carried by a Cloud collection yet compare against it instead of Cloud's base.
 */
export type SectionSnapshot = Record<string, JsonValue>;

export function readSnapshot(scope: string): SectionSnapshot | null {
  try {
    const raw = appStorage().getItem(PREFIX + scope);
    if (raw === null) return null;
    const parsed: unknown = JSON.parse(raw);
    return parsed !== null && typeof parsed === 'object' && !Array.isArray(parsed) ? (parsed as SectionSnapshot) : null;
  } catch {
    return null;
  }
}

export function writeSnapshot(scope: string, snapshot: SectionSnapshot): void {
  try {
    appStorage().setItem(PREFIX + scope, JSON.stringify(snapshot));
  } catch {
    // Storage can be blocked or full; the section then starts a fresh snapshot next time.
  }
}

const revision = shallowRef(0);

/**
 * A counter that grows each time a reference was advanced on purpose (`advanceSnapshot`). A reactive
 * source (a button's count, an open panel) reads it so it lists again, because nothing about the
 * records themselves changed.
 */
export function snapshotRevision(): number {
  return revision.value;
}

/** Replaces the reference copy of a section with `snapshot` and tells every reader to compare again. */
export function advanceSnapshot(scope: string, snapshot: SectionSnapshot): void {
  writeSnapshot(scope, snapshot);
  revision.value += 1;
}
