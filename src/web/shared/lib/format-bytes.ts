const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

/** Human-readable size in the given locale, e.g. `1,5 MB`. */
export function formatBytes(value: number, locale: string): string {
  let amount = Math.max(0, value);
  let unit = 0;
  while (amount >= 1024 && unit < UNITS.length - 1) {
    amount /= 1024;
    unit += 1;
  }
  const digits = unit === 0 ? 0 : 1;
  const number = new Intl.NumberFormat(locale, { maximumFractionDigits: digits, minimumFractionDigits: digits });
  return `${number.format(amount)} ${UNITS[unit]}`;
}
