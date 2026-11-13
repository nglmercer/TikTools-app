import { configureAppStorage } from '@/shared/lib/app-storage.ts';

export function memoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() { return data.size; },
    clear: () => data.clear(),
    getItem: (key) => data.get(key) ?? null,
    key: (index) => [...data.keys()][index] ?? null,
    removeItem: (key) => { data.delete(key); },
    setItem: (key, value) => { data.set(key, String(value)); },
  };
}

/** Same-origin frames share physical storage; each demo instance owns only its prefix. */
export function namespacedStorage(storage: Storage, instance: string): Storage {
  if (!/^[a-zA-Z0-9_-]{1,64}$/.test(instance)) throw new Error('Invalid demo storage instance.');
  const prefix = `tiktools.demo.${instance}.`;
  const keys = () => Array.from({ length: storage.length }, (_, index) => storage.key(index)).filter((key): key is string => key !== null && key.startsWith(prefix));
  return {
    get length() { return keys().length; },
    clear: () => { for (const key of keys()) storage.removeItem(key); },
    getItem: (key) => storage.getItem(prefix + key),
    key: (index) => keys()[index]?.slice(prefix.length) ?? null,
    removeItem: (key) => storage.removeItem(prefix + key),
    setItem: (key, value) => {
      if (key.length > 512 || value.length > 2 * 1024 * 1024) throw new Error('Demo storage limit exceeded.');
      const owned = keys();
      if ((!owned.includes(prefix + key) && owned.length >= 128) || owned.reduce((size, name) => size + (name === prefix + key ? 0 : storage.getItem(name)?.length ?? 0), value.length) > 4 * 1024 * 1024) throw new Error('Demo storage capacity exceeded.');
      storage.setItem(prefix + key, value);
    },
  };
}

export function installDemoStorage(instance: string, backing: { local: Storage; session: Storage }) {
  const local = namespacedStorage(backing.local, instance);
  const session = namespacedStorage(backing.session, instance);
  configureAppStorage((kind) => kind === 'local' ? local : session);
  return { clear: () => { local.clear(); session.clear(); } };
}
