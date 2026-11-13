/** Native entries keep browser storage. Explicit demo bootstrap may supply an isolated adapter. */
export type AppStorageKind = 'local' | 'session';
let adapter: ((kind: AppStorageKind) => Storage) | undefined;

export function configureAppStorage(next: ((kind: AppStorageKind) => Storage) | undefined): void {
  adapter = next;
}

export function appStorage(kind: AppStorageKind = 'local'): Storage {
  if (adapter) return adapter(kind);
  return kind === 'local' ? globalThis.localStorage : globalThis.sessionStorage;
}
