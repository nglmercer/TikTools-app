import { afterEach, describe, expect, it } from 'vitest';
import { configureAppStorage } from '@/shared/lib/app-storage.ts';
import { loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import { storeLocale, storedLocale } from '@/shared/i18n/locale-preference.ts';
import { installDemoStorage, memoryStorage, namespacedStorage } from './storage.ts';

afterEach(() => configureAppStorage(undefined));

describe('isolated demo preferences', () => {
  it('uses the real fixed-key helpers without reading or changing ordinary app preferences', () => {
    const local = memoryStorage();
    local.setItem('tiktok-live-locale', 'fr');
    local.setItem('tiktok-live-view-state', JSON.stringify({ version: 1, tab: 'plugins' }));
    installDemoStorage('first', { local, session: memoryStorage() });
    expect(storedLocale()).toBeNull();
    expect(loadViewState()).toEqual({});
    storeLocale('es');
    saveViewState('feed', { searchQuery: 'Fictional viewer' });
    expect(local.getItem('tiktok-live-locale')).toBe('fr');
    expect(local.getItem('tiktools.demo.first.tiktok-live-locale')).toBe('es');
    expect(loadViewState().feed?.searchQuery).toBe('Fictional viewer');
  });
  it('resets one iframe prefix while preserving sibling frames and unrelated storage', () => {
    const storage = memoryStorage();
    storage.setItem('ordinary', 'keep');
    const a = namespacedStorage(storage, 'a');
    const b = namespacedStorage(storage, 'b');
    a.setItem('journal', 'one');
    b.setItem('journal', 'two');
    expect(a.length).toBe(1);
    expect(a.key(0)).toBe('journal');
    a.clear();
    expect(b.getItem('journal')).toBe('two');
    expect(storage.getItem('ordinary')).toBe('keep');
    expect(a.getItem('journal')).toBeNull();
  });
  it('rejects invalid namespaces before creating an adapter', () => {
    expect(() => namespacedStorage(memoryStorage(), '../native')).toThrow('Invalid demo storage instance');
  });
});
