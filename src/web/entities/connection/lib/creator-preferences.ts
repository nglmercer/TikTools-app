import { appStorage } from '@/shared/lib/app-storage.ts';
const USERNAME_KEY = 'tiktok-live-username';
const RECENT_KEY = 'tiktok-live-recents';
const MAX_RECENTS = 5;

function readPreference(key: string): string | null {
  try {
    return appStorage().getItem(key);
  } catch {
    return null;
  }
}

function writePreference(key: string, value: string): void {
  try {
    appStorage().setItem(key, value);
  } catch {
    return;
  }
}

function normalize(username: string): string {
  return username.trim().replace(/^@/, '');
}

export function getSavedUsername(): string {
  return normalize(readPreference(USERNAME_KEY) ?? '');
}

export function getRecentUsernames(): string[] {
  const stored = readPreference(RECENT_KEY);
  if (!stored) return [];
  try {
    const list: unknown = JSON.parse(stored);
    return Array.isArray(list) ? list.filter((entry): entry is string => typeof entry === 'string' && entry.trim().length > 0) : [];
  } catch {
    return [];
  }
}

export function addRecentUsername(username: string): string[] {
  const clean = normalize(username);
  if (!clean) return getRecentUsernames();
  const updated = [clean, ...getRecentUsernames().filter((entry) => entry.toLowerCase() !== clean.toLowerCase())].slice(0, MAX_RECENTS);
  writePreference(RECENT_KEY, JSON.stringify(updated));
  return updated;
}

export function saveUsername(username: string): void {
  const clean = normalize(username);
  writePreference(USERNAME_KEY, clean);
  if (clean) addRecentUsername(clean);
}
