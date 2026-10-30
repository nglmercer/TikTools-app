
import { parsePluginNavId } from '../../../automation/plugins/declarative.ts';
import type { BuiltinAppTab, EventFilter } from '@/types.ts';

const STORAGE_KEY = 'tiktok-live-view-state';
const STORAGE_VERSION = 1;
const MAX_STRING = 256;
const MAX_TAB = 128;
const MAX_SCROLL_TOP = 10_000_000;
const MAX_OPEN_SERVERS = 64;

const BUILTIN_TABS: readonly BuiltinAppTab[] = [
  'feed',
  'points',
  'analytics',
  'connect',
  'behavior',
  'plugins',
  'widgets',
  'rankings',
  'settings',
];

export interface FeedViewState {
  filter?: EventFilter;
  searchQuery?: string;
  autoScroll?: boolean;
  scrollTop?: number;
}

export interface BehaviorViewState {
  actionQuery?: string;
  actionSort?: 'name' | 'name-desc' | 'enabled' | 'disabled';
  eventQuery?: string;
  eventSort?: 'name' | 'name-desc' | 'enabled' | 'disabled';
  tab?: 'events' | 'actions';
  selected?: string;
}

export interface PointsViewState {
  searchQuery?: string;
  sortBy?: 'points' | 'level' | 'viewer';
  sortDir?: 'asc' | 'desc';
}

export interface AnalyticsViewState {
  range?: 'today' | '7d' | '30d' | '90d' | 'custom';
  metric?: 'chats' | 'gifts' | 'likes' | 'diamonds' | 'peakViewers';
  tab?: 'overview' | 'engagement' | 'contributors' | 'sessions';
  contributorQuery?: string;
  customStart?: string;
  customEnd?: string;
}

export interface WidgetsViewState {
  selectedKind?: 'follow' | 'gift' | 'chat' | 'share' | 'subscribe' | 'likes' | 'ranking';
}

export interface PluginsViewState {
  view?: 'plugins' | 'processors';
  catalog?: 'installed' | 'available';
  query?: string;
  selected?: string;
  details?: 'general' | 'settings' | 'connection';
}

export interface ConnectionsViewState {
  showCookie?: boolean;
  openServers?: Record<string, boolean>;
}

export interface AsideViewState {
  open?: boolean;
}

export interface SidebarViewState {
  collapsed?: boolean;
}

export interface ViewState {
  tab?: string;
  feed?: FeedViewState;
  behavior?: BehaviorViewState;
  points?: PointsViewState;
  analytics?: AnalyticsViewState;
  widgets?: WidgetsViewState;
  plugins?: PluginsViewState;
  connections?: ConnectionsViewState;
  aside?: AsideViewState;
  sidebar?: SidebarViewState;
}

function readRaw(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function writeRaw(value: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, value);
  } catch {
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function pickDefined<T extends Record<string, unknown>>(source: T): Partial<T> {
  const result: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(source)) {
    if (value !== undefined) result[key] = value;
  }
  return result as Partial<T>;
}

function cleanString(value: unknown): string | undefined {
  if (typeof value !== 'string') return undefined;
  const trimmed = value.slice(0, MAX_STRING);
  return trimmed === '' ? undefined : trimmed;
}

function cleanDate(value: unknown): string | undefined {
  if (typeof value !== 'string') return undefined;
  return /^\d{4}-\d{2}-\d{2}$/.test(value) ? value : undefined;
}

function cleanScrollTop(value: unknown): number | undefined {
  if (typeof value !== 'number' || !Number.isFinite(value)) return undefined;
  const clamped = Math.min(Math.max(Math.round(value), 0), MAX_SCROLL_TOP);
  return clamped === 0 ? undefined : clamped;
}

function cleanBoolean(value: unknown): boolean | undefined {
  return typeof value === 'boolean' ? value : undefined;
}

function oneOf<T extends string>(value: unknown, allowed: readonly T[]): T | undefined {
  return typeof value === 'string' && (allowed as readonly string[]).includes(value)
    ? (value as T)
    : undefined;
}

function cleanTab(value: unknown): string | undefined {
  if (typeof value !== 'string' || value.length > MAX_TAB) return undefined;
  if ((BUILTIN_TABS as readonly string[]).includes(value)) return value;
  return parsePluginNavId(value) ? value : undefined;
}

function cleanFeed(value: unknown): FeedViewState | undefined {
  if (!isRecord(value)) return undefined;
  const cleaned = pickDefined({
    filter: oneOf(value['filter'], ['all', 'chat', 'gift', 'like', 'social']),
    searchQuery: cleanString(value['searchQuery']),
    autoScroll: cleanBoolean(value['autoScroll']),
    scrollTop: cleanScrollTop(value['scrollTop']),
  });
  return Object.keys(cleaned).length > 0 ? (cleaned as FeedViewState) : undefined;
}

function cleanBehavior(value: unknown): BehaviorViewState | undefined {
  if (!isRecord(value)) return undefined;
  const cleaned = pickDefined({
    actionQuery: cleanString(value['actionQuery']),
    actionSort: oneOf(value['actionSort'], ['name', 'name-desc', 'enabled', 'disabled']),
    eventQuery: cleanString(value['eventQuery']),
    eventSort: oneOf(value['eventSort'], ['name', 'name-desc', 'enabled', 'disabled']),
    tab: oneOf(value['tab'], ['events', 'actions']),
    selected: cleanString(value['selected']),
  });
  return Object.keys(cleaned).length > 0 ? (cleaned as BehaviorViewState) : undefined;
}

function cleanPoints(value: unknown): PointsViewState | undefined {
  if (!isRecord(value)) return undefined;
  const cleaned = pickDefined({
    searchQuery: cleanString(value['searchQuery']),
    sortBy: oneOf(value['sortBy'], ['points', 'level', 'viewer']),
    sortDir: oneOf(value['sortDir'], ['asc', 'desc']),
  });
  return Object.keys(cleaned).length > 0 ? (cleaned as PointsViewState) : undefined;
}

function cleanAnalytics(value: unknown): AnalyticsViewState | undefined {
  if (!isRecord(value)) return undefined;
  const cleaned = pickDefined({
    range: oneOf(value['range'], ['today', '7d', '30d', '90d', 'custom']),
    metric: oneOf(value['metric'], ['chats', 'gifts', 'likes', 'diamonds', 'peakViewers']),
    tab: oneOf(value['tab'], ['overview', 'engagement', 'contributors', 'sessions']),
    contributorQuery: cleanString(value['contributorQuery']),
    customStart: cleanDate(value['customStart']),
    customEnd: cleanDate(value['customEnd']),
  });
  return Object.keys(cleaned).length > 0 ? (cleaned as AnalyticsViewState) : undefined;
}

function cleanWidgets(value: unknown): WidgetsViewState | undefined {
  if (!isRecord(value)) return undefined;
  const cleaned = pickDefined({
    selectedKind: oneOf(value['selectedKind'], ['follow', 'gift', 'chat', 'share', 'subscribe', 'likes', 'ranking']),
  });
  return Object.keys(cleaned).length > 0 ? (cleaned as WidgetsViewState) : undefined;
}

function cleanPlugins(value: unknown): PluginsViewState | undefined {
  if (!isRecord(value)) return undefined;
  const cleaned = pickDefined({
    view: oneOf(value['view'], ['plugins', 'processors']),
    catalog: oneOf(value['catalog'], ['installed', 'available']),
    query: cleanString(value['query']),
    selected: cleanString(value['selected']),
    details: oneOf(value['details'], ['general', 'settings', 'connection']),
  });
  return Object.keys(cleaned).length > 0 ? (cleaned as PluginsViewState) : undefined;
}

function cleanConnections(value: unknown): ConnectionsViewState | undefined {
  if (!isRecord(value)) return undefined;
  const cleaned = pickDefined({
    showCookie: cleanBoolean(value['showCookie']),
    openServers: cleanOpenServers(value['openServers']),
  });
  return Object.keys(cleaned).length > 0 ? (cleaned as ConnectionsViewState) : undefined;
}

function cleanAside(value: unknown): AsideViewState | undefined {
  if (!isRecord(value)) return undefined;
  const cleaned = pickDefined({ open: cleanBoolean(value['open']) });
  return Object.keys(cleaned).length > 0 ? (cleaned as AsideViewState) : undefined;
}

function cleanSidebar(value: unknown): SidebarViewState | undefined {
  if (!isRecord(value)) return undefined;
  const cleaned = pickDefined({ collapsed: cleanBoolean(value['collapsed']) });
  return Object.keys(cleaned).length > 0 ? (cleaned as SidebarViewState) : undefined;
}

function cleanOpenServers(value: unknown): Record<string, boolean> | undefined {
  if (!isRecord(value)) return undefined;
  const result: Record<string, boolean> = {};
  for (const [server, open] of Object.entries(value)) {
    if (Object.keys(result).length >= MAX_OPEN_SERVERS) break;
    if (server.length > MAX_TAB || typeof open !== 'boolean') continue;
    result[server] = open;
  }
  return Object.keys(result).length > 0 ? result : undefined;
}

export function loadViewState(): ViewState {
  const raw = readRaw();
  if (!raw) return {};
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return {};
  }
  if (!isRecord(parsed) || parsed['version'] !== STORAGE_VERSION) return {};
  const state = pickDefined({
    tab: cleanTab(parsed['tab']),
    feed: cleanFeed(parsed['feed']),
    behavior: cleanBehavior(parsed['behavior']),
    points: cleanPoints(parsed['points']),
    analytics: cleanAnalytics(parsed['analytics']),
    widgets: cleanWidgets(parsed['widgets']),
    plugins: cleanPlugins(parsed['plugins']),
    connections: cleanConnections(parsed['connections']),
    aside: cleanAside(parsed['aside']),
    sidebar: cleanSidebar(parsed['sidebar']),
  });
  return state as ViewState;
}

export function saveViewState<K extends keyof ViewState>(
  section: K,
  value: NonNullable<ViewState[K]>,
): void {
  let stored: Record<string, unknown> = {};
  const raw = readRaw();
  if (raw) {
    try {
      const parsed: unknown = JSON.parse(raw);
      if (isRecord(parsed)) stored = parsed;
    } catch {
      stored = {};
    }
  }
  stored['version'] = STORAGE_VERSION;
  stored[section] = value;
  writeRaw(JSON.stringify(stored));
}

export type DebouncedCallback = (() => void) & { cancel: () => void };

export function debounce(fn: () => void, ms = 150): DebouncedCallback {
  let timer: ReturnType<typeof setTimeout> | null = null;
  const cancel = (): void => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
  };
  const callback = (): void => {
    cancel();
    timer = setTimeout(() => {
      timer = null;
      fn();
    }, ms);
  };
  return Object.assign(callback, { cancel });
}
