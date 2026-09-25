import type {
  HotkeyBackendCapabilities,
  HotkeyBackendStatus,
  HotkeyStatusData,
} from '../../../shared/messages.ts';

export type { HotkeyBackendCapabilities, HotkeyBackendStatus, HotkeyStatusData } from '../../../shared/messages.ts';

/**
 * Capability-aware helpers for the hotkey process plugin's `hotkey.status`
 * events. The plugin emits one status event per backend change with
 * per-backend `state` (`starting`, `active`, `permission required`,
 * `unsupported`, `failed`) and a capability map, so the UI can render
 * listener health and warn when a `sequence contains` behavior has no
 * backend able to observe sequences (Wayland without raw-input permission).
 */

export interface HotkeyStatusSummary {
  /** One-line headline, e.g. `Global Hotkeys: active via XDG Desktop Portal`. */
  headline: string;
  /** At least one backend observes chords. */
  chordsSupported: boolean;
  /** At least one backend observes arbitrary keys/sequences. */
  sequencesSupported: boolean;
  /** A backend is stuck on permissions (evdev group, macOS Accessibility). */
  needsPermission: boolean;
  /** Whether the UI should interrupt the quiet Behavior screen with status. */
  needsAttention: boolean;
  /** Per-backend display lines. */
  lines: string[];
}

const ACTIVE = new Set(['active', 'running']);
const PERMISSION = new Set(['permission required', 'permission-required']);

function capsOf(entry: HotkeyBackendStatus): HotkeyBackendCapabilities {
  // Backends without an explicit capability map predate it; only the portal
  // lacks arbitrary-key observation.
  const fallback = entry.backend !== 'portal';
  return {
    globalChords: entry.capabilities?.globalChords ?? fallback,
    arbitraryKeys: entry.capabilities?.arbitraryKeys ?? fallback,
    sequences: entry.capabilities?.sequences ?? fallback,
    keyRelease: entry.capabilities?.keyRelease ?? fallback,
  };
}

export type HotkeyListenerState =
  | 'active'
  | 'starting'
  | 'permission'
  | 'failed'
  | 'unsupported'
  | 'unknown';

/** Single listener state for the always-visible status panel. */
export function hotkeyListenerState(
  data: HotkeyStatusData | null | undefined,
): HotkeyListenerState {
  if (!data || !Array.isArray(data.backends) || data.backends.length === 0) return 'unknown';
  const states = new Set(data.backends.map((entry) => entry.state));
  if (data.backends.some((entry) => PERMISSION.has(entry.state))) return 'permission';
  if (data.backends.some((entry) => entry.state === 'failed')) return 'failed';
  if ([...states].every((state) => state === 'unsupported')) return 'unsupported';
  if (data.backends.some((entry) => ACTIVE.has(entry.state))) return 'active';
  return 'starting';
}

/**
 * True when a backend failed for the one reason the UI can fix in-app:
 * unreadable `/dev/input` devices (Linux/Wayland seat permissions). The
 * native denial always names `/dev/input`; every other failure (display
 * errors, unsupported sessions) needs a different fix, so the grant
 * button stays hidden for those.
 */
export function hotkeyNeedsSeatAccess(
  data: HotkeyStatusData | null | undefined,
): boolean {
  if (!data || !Array.isArray(data.backends)) return false;
  return data.backends.some(
    (entry) =>
      (entry.state === 'failed' || PERMISSION.has(entry.state)) &&
      (entry.detail || '').includes('/dev/input'),
  );
}

/** Human chord for the last received press, e.g. `Ctrl+K`. */
export function formatHotkeyChord(key: string, modifiers: string): string {
  const parts = [...modifiers.split('+').filter(Boolean), key].filter(Boolean);
  return parts
    .map((part) => (part.length <= 1 ? part.toUpperCase() : part.charAt(0).toUpperCase() + part.slice(1)))
    .join('+');
}

export function summarizeHotkeyStatus(data: HotkeyStatusData | null | undefined): HotkeyStatusSummary {
  if (!data || !Array.isArray(data.backends) || data.backends.length === 0) {
    return {
      headline: 'Global Hotkeys: status unknown',
      chordsSupported: false,
      sequencesSupported: false,
      needsPermission: false,
      needsAttention: false,
      lines: [],
    };
  }
  const running = data.backends.filter((entry) => ACTIVE.has(entry.state));
  const chordsSupported = running.some((entry) => capsOf(entry).globalChords);
  const sequencesSupported = running.some((entry) => capsOf(entry).sequences);
  const needsPermission = data.backends.some((entry) => PERMISSION.has(entry.state));
  const isConfigDisabled = (entry: HotkeyBackendStatus): boolean =>
    entry.state === 'unsupported' && (entry.detail || '').includes('sequencesNeeded=false');
  const actionable = data.backends.filter((entry) => {
    if (PERMISSION.has(entry.state)) return true;
    if (isConfigDisabled(entry)) return false;
    return entry.state === 'failed' || entry.state === 'unsupported';
  });
  const uniqueLines = [...new Set(data.backends.map((entry) => entry.summary || `${entry.backend}: ${entry.state}`))];
  const headlineEntry = actionable[0] ?? running[0] ?? data.backends[0];
  const headline = headlineEntry
    ? (headlineEntry.summary || `Global Hotkeys: ${headlineEntry.state} via ${headlineEntry.backend}`)
    : 'Global Hotkeys: unavailable';
  return {
    headline,
    chordsSupported,
    sequencesSupported,
    needsPermission,
    needsAttention: actionable.length > 0
      || (running.length === 0 && data.backends.some((entry) => entry.state !== 'starting' && !isConfigDisabled(entry))),
    lines: uniqueLines,
  };
}

/**
 * What to show next to a behavior that filters on `event.data.sequence`.
 * `requested` is whether the behavior uses a sequence filter at all.
 */
export function sequenceTriggerHint(
  status: HotkeyStatusSummary,
  requested: boolean,
): string | null {
  if (!requested) return null;
  if (status.sequencesSupported) return null;
  if (status.needsPermission) {
    return 'This trigger needs raw keyboard access on Wayland. Approve the automatic system authorization request to enable sequence triggers.';
  }
  return 'No running backend observes key sequences on this session.';
}

/**
 * Returns true when a hotkey behavior needs raw keyboard observation rather
 * than a portal-registered chord. A bare key such as `a`, a sequence filter,
 * or a non-portal backend filter cannot be represented by BindShortcuts.
 */
export function requiresRawHotkeyInput(
  filters: Array<{ path: string; operator: string; value: string }>,
): boolean {
  const key = filters.find(
    (filter) => filter.path === 'event.data.key' && filter.operator === 'eq' && filter.value.trim() !== '',
  );
  const modifiers = filters.find(
    (filter) => filter.path === 'event.data.modifiers' && filter.operator === 'eq',
  );
  if (!key || !modifiers || modifiers.value.trim() === '') return true;
  if (filters.some((filter) => filter.path === 'event.data.sequence')) return true;
  return filters.some((filter) => (
    filter.path === 'event.data.backend'
    && !(filter.operator === 'eq' && filter.value.trim().toLowerCase() === 'portal')
  ));
}

/**
 * The warning is only useful for a native Wayland session where the portal
 * cannot observe arbitrary keys. X11 and active raw-input backends need no
 * warning, even when the behavior is a bare-key trigger.
 */
export function rawHotkeyInputHint(
  data: HotkeyStatusData | null | undefined,
  requested: boolean,
): string | null {
  if (!requested || data?.session !== 'wayland') return null;
  const summary = summarizeHotkeyStatus(data);
  if (summary.sequencesSupported) return null;
  if (summary.needsPermission) {
    return 'This trigger needs raw keyboard access on Wayland. Approve the automatic system authorization request to enable arbitrary key triggers.';
  }
  return 'This trigger needs raw keyboard access on Wayland. TikTools will request it automatically when the behavior is enabled.';
}
