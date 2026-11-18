/** One row of a comparison table shared by automation and library reviews.
 * `before`/`after` are `unknown` because host records are typed interfaces
 * without an index signature; the table casts them to JsonValue at the edge. */
export interface ReviewRow {
  /** Stable key used for expansion state. */
  id: string;
  /** Human readable record / version name. */
  name: string;
  /** event | action | version | diff — also used for the Type column. */
  kind?: 'event' | 'action' | 'version' | 'diff';
  change?: 'add' | 'remove' | 'change' | 'same' | 'version' | 'diff';
  /** Optional saved revision, shown under the name. */
  revision?: string;
  /** Short note shown under the name, e.g. "new record" / "deleted on host". */
  note?: string;
  before?: unknown;
  after?: unknown;
  /** New records can be renamed before applying the change. */
  editable?: boolean;
}

export function reviewRowOpenKey(row: ReviewRow, index = 0): string {
  return row.id || String(index);
}
