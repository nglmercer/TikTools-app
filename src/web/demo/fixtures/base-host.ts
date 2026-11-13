/** Data-only host fixtures shared by the public demo and browser tests. No IPC globals or Playwright. */
import type { VariableComparison } from '@/features/edit-runtime-globals/model/variable-sync-adapter.ts';
import type { RuntimeVariable } from '@/entities/runtime-global/lib/runtime-variable.ts';
import type { CollaborationStatus, Invitation, InvitationSecret, PairingRequest, Principal, Session, AuditEntry } from '@/entities/collaboration/api/types.ts';

export interface MemoryHostState {
  collaboration?: {
    status: CollaborationStatus;
    invitations: Invitation[]; issued: InvitationSecret; requests: PairingRequest[];
    principals: Principal[]; sessions: Session[]; audit: AuditEntry[];
  };
  /** Methods announced through `rpc.discover`; none by default, so optional Cloud areas stay hidden. */
  announcedMethods?: string[];
  /** Cloud account and v2 workspace served to the sync panel (`cloud.status`, `sync.status`, `sync.v2.*`). */
  sync?: {
    signedIn: boolean;
    enabled: boolean;
    lastSyncedAt?: number | null;
    entries: Array<Record<string, unknown>>;
    /** Conflict handles resolved so far, as `<collection>/<key>:<choice>`. */
    resolved: string[];
  };
  /**
   * Cloud catalog served by `catalog.items` / `catalog.mine` / `catalog.item` (wire items). `error` makes
   * the list methods fail with that code and optional `data`; `dataState` is forwarded as the page's.
   */
  catalog?: {
    items: Array<Record<string, unknown>>;
    mine?: Array<Record<string, unknown>>;
    /** Documents served by `cloudLibrary.list` (the Cloud tab of the rule templates dialog). */
    library?: Array<Record<string, unknown>>;
    dataState?: 'ready' | 'noActiveGeneration';
    error?: { code: string; message: string; data?: unknown };
  };
  /** Runtime globals served by `globals.list` and written by `globals.set`. */
  globals?: Record<string, string>;
  typedGlobals?: Record<string, RuntimeVariable>;
  typedGlobalsRevision?: string;
  variableComparisons?: VariableComparison[];
  /** Behavior snapshot served for `automation.snapshot`. */
  automationSnapshot: Record<string, unknown>;
  automationVersions?: Record<string, string>;
  automationDelayMs?: number;
  automationRuns: Array<Record<string, unknown>>;
  pointsConfig: Record<string, unknown>;
  leaderboard: Array<Record<string, unknown>>;
  creator: Record<string, unknown> | null;
  recentCreators: Array<Record<string, unknown>>;
  gifts: Array<Record<string, unknown>>;
  processors: Array<Record<string, unknown>>;
  /** `plugins.settings.get/set` store, keyed by plugin id. */
  pluginSettings: Record<
    string,
    { schema: Record<string, unknown>; uiHints?: Record<string, unknown>; values: Record<string, unknown> }
  >;
  /** `plugins.options` store, keyed by source id. */
  pluginOptions: Record<string, { options: Array<{ value: string; label: string }>; selected?: string | null }>;
  /** `plugins.health` results, keyed by plugin id. */
  pluginHealth: Record<string, { ok: boolean; latencyMs: number; error?: string | null; reachable?: boolean; gameplayReady?: boolean }>;
  pluginTokenCandidates?: Record<string, Array<{ id: string; label: string }>>;
  pluginSetupEndpoints?: Record<string, string>;
  pluginSetupHealth?: MemoryHostState['pluginHealth'];
  /** `plugins.action.execute` results, keyed by action type. */
  actionOutcomes: Record<
    string,
    { ok: boolean; summary: string; logs?: string[]; error?: string | null }
  >;
  /** `app.state.*` key/value store. */
  appState: Record<string, string>;
  /** Authoritative `live.status`/`live.connect`/`live.disconnect` result. */
  liveStatus: {
    connected: boolean;
    uniqueId: string | null;
    roomId: string | null;
    connectionId: string | null;
    native: boolean;
  };
  /** Page served for `live.channels` (read-only LIVE browse). */
  liveMonitor?: Record<string, unknown>;
  feedJournal?: Array<{ cursor: number; event: Record<string, unknown> }>;
  liveChannels: { rooms: Array<Record<string, unknown>>; nextOffset?: number };
  /** Methods forced to fail: method -> message, or `{ code, message }` to
   *  exercise a specific JSON-RPC error code. */
  failures: Record<string, string | { code: string; message: string }>;
  /** Artificial delay (ms) for `plugins.action.execute` responses. */
  actionDelayMs: number;
  /** Every RPC call received, in order. */
  calls: Array<{ method: string; params: Record<string, unknown>; meta?: Record<string, unknown> }>;
  /** Calls with no fake handler — must stay empty. */
  unhandled: string[];
  /** Legacy `type:` messages (plugin-ui-open/close), in order. */
  legacyMessages: Array<Record<string, unknown>>;
  frontendReady: boolean;
}

export function baseHostState(overrides: Partial<MemoryHostState> = {}): MemoryHostState {
  return {
    automationSnapshot: {
      actions: [],
      events: [],
      plugins: [],
      actionTypes: [],
      eventTypes: [],
      pluginTemplates: [],
      pluginPages: [],
      translations: {},
    },
    automationRuns: [],
    pointsConfig: {
      currencyName: 'Points',
      pointsPerCoin: 1.0,
      pointsPerCoinEnabled: true,
      pointsPerShare: 3.0,
      pointsPerShareEnabled: true,
      pointsPerChat: 1.0,
      pointsPerChatEnabled: true,
      pointsPerLike: 0.1,
      pointsPerLikeEnabled: true,
      pointsPerFollow: 5.0,
      pointsPerFollowEnabled: true,
      pointsPerJoin: 0.5,
      pointsPerJoinEnabled: false,
      subBonusMultiplier: 0.0,
      pointsPerLevel: 100,
    },
    leaderboard: [],
    creator: null,
    recentCreators: [],
    gifts: [],
    processors: [],
    pluginSettings: {},
    pluginOptions: {},
    pluginHealth: {},
    actionOutcomes: {},
    appState: {},
    liveStatus: {
      connected: false,
      uniqueId: null,
      roomId: null,
      connectionId: null,
      native: true,
    },
    liveChannels: { rooms: [] },
    failures: {},
    actionDelayMs: 0,
    calls: [],
    unhandled: [],
    legacyMessages: [],
    frontendReady: false,
    ...overrides,
  };
}
