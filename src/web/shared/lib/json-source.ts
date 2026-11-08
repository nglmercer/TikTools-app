/** A browser cannot claim an exact source write after rounding a Rust integer. */
export function sourceNumbersSafe(source: unknown): boolean {
  const pending = [source];
  while (pending.length) {
    const value = pending.pop();
    if (typeof value === 'number' && (!Number.isFinite(value) || Number.isInteger(value) && !Number.isSafeInteger(value))) return false;
    if (value && typeof value === 'object') for (const child of Object.values(value)) pending.push(child);
  }
  return true;
}
