/**
 * Capability-aware helpers for the hotkey process plugin's `hotkey.status`
 * events. The plugin emits one status event per backend change with
 * per-backend `state` (`starting`, `active`, `permission required`,
 * `unsupported`, `failed`) and a capability map, so the UI can render
 * listener health and warn when a `sequence contains` behavior has no
 * backend able to observe sequences (Wayland without raw-input permission).
 */

export interface HotkeyBackendCapabilities {
  globalChords: boolean;
  arbitraryKeys: boolean;
  sequences: boolean;
  keyRelease: boolean;
}

export interface HotkeyBackendStatus {
  backend: string;
  state: string;
  detail: string;
  summary: string;
  capabilities?: Partial<HotkeyBackendCapabilities>;
}

export interface HotkeyStatusData {
  platform: string;
  session: string;
  backends: HotkeyBackendStatus[];
}

export interface HotkeyStatusSummary {
  /** One-line headline, e.g. `Global Hotkeys: active via XDG Desktop Portal`. */
  headline: string;
  /** At least one backend observes chords. */
  chordsSupported: boolean;
  /** At least one backend observes arbitrary keys/sequences. */
  sequencesSupported: boolean;
  /** A backend is stuck on permissions (evdev group, macOS Accessibility). */
  needsPermission: boolean;
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

export function summarizeHotkeyStatus(data: HotkeyStatusData | null | undefined): HotkeyStatusSummary {
  if (!data || !Array.isArray(data.backends) || data.backends.length === 0) {
    return {
      headline: 'Global Hotkeys: status unknown',
      chordsSupported: false,
      sequencesSupported: false,
      needsPermission: false,
      lines: [],
    };
  }
  const running = data.backends.filter((entry) => ACTIVE.has(entry.state));
  const chordsSupported = running.some((entry) => capsOf(entry).globalChords);
  const sequencesSupported = running.some((entry) => capsOf(entry).sequences);
  const needsPermission = data.backends.some((entry) => PERMISSION.has(entry.state));
  const headline =
    running.length > 0
      ? (running[0]?.summary || `Global Hotkeys: active via ${running[0]?.backend}`)
      : (data.backends.find((entry) => PERMISSION.has(entry.state))?.summary
        || data.backends[0]?.summary
        || 'Global Hotkeys: unavailable');
  return {
    headline,
    chordsSupported,
    sequencesSupported,
    needsPermission,
    lines: data.backends.map((entry) => entry.summary || `${entry.backend}: ${entry.state}`),
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
    return 'This trigger requires raw keyboard access on Wayland. Grant input-device permission to enable sequence triggers.';
  }
  return 'No running backend observes key sequences on this session.';
}
