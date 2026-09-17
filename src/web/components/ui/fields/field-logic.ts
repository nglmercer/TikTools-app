/**
 * Pure foundation for the shared field system (`ui/fields/`).
 *
 * Everything here is DOM-free so Phase 3 (autocomplete controller, template
 * insertion, popover positioning) can reuse it and bun can test it directly.
 * Components must derive ids, sizes, and number drafts through these helpers
 * instead of re-implementing them, so shell, control, and message agree.
 */

export type FieldSize = 'sm' | 'md' | 'lg';

/** Resolve any input to a valid size. The system default is `md` (44-48px). */
export function resolveFieldSize(size: unknown): FieldSize {
  return size === 'sm' || size === 'md' || size === 'lg' ? size : 'md';
}

/**
 * One id rule for every field: explicit `id` wins, otherwise derive from
 * `name` so labels and ARIA references stay stable without caller effort.
 */
export function fieldControlId(props: { id?: string; name?: string }, fallback: string): string {
  if (props.id) return props.id;
  if (props.name) return `tt-${props.name}`;
  return fallback;
}

/** Join rendered message ids for `aria-describedby`. Empty when none exist. */
export function describeField(parts: Array<string | undefined | false>): string | undefined {
  const list = parts.filter((entry): entry is string => typeof entry === 'string' && entry.length > 0);
  return list.length > 0 ? list.join(' ') : undefined;
}

export type FieldMessageIds = {
  descriptionId: string;
  errorId: string;
};

/**
 * Ids for the visible help/error lines. FieldShell renders these ids and the
 * control references them — both sides must call this helper with the same
 * control id so `aria-describedby`/`aria-errormessage` never dangle.
 */
export function fieldMessageIds(controlId: string): FieldMessageIds {
  return {
    descriptionId: `${controlId}-description`,
    errorId: `${controlId}-error`,
  };
}

export type NumberDraft = { kind: 'empty' } | { kind: 'invalid' } | { kind: 'number'; value: number };

/**
 * Classify raw number keystrokes without committing: intermediate states
 * (`''`, `'-'`, `'.'`) stay editable and emit `null` instead of clamping.
 */
export function parseNumberDraft(raw: string): NumberDraft {
  const text = raw.trim();
  if (text === '' || text === '-' || text === '.' || text === '-.') return { kind: 'empty' };
  const parsed = Number(raw);
  if (Number.isNaN(parsed)) return { kind: 'invalid' };
  return { kind: 'number', value: parsed };
}

/** Round a value to the precision implied by `step` (defaults to integer). */
export function roundToStep(value: number, step?: number): number {
  if (!step || step >= 1) return Math.round(value);
  const decimals = String(step).split('.')[1]?.length ?? 2;
  return Number(value.toFixed(decimals));
}
