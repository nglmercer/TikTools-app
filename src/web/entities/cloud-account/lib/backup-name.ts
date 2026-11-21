const pad = (value: number): string => String(value).padStart(2, '0');

/** A readable, sortable default name such as `Backup 2026-11-21 02:50`, in the user's local time. */
export function suggestBackupName(prefix: string, date: Date = new Date()): string {
  const day = `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
  return `${prefix} ${day} ${pad(date.getHours())}:${pad(date.getMinutes())}`.slice(0, 120);
}
