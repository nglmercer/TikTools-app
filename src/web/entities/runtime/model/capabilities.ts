/**
 * Method availability for one host, connection and session.
 *
 * A method is `true` (announced by the host), `false` (absent from a current
 * snapshot) or `undefined` (no current snapshot yet). Callers must treat
 * `undefined` as "unknown, do not assume either way" and must never report an
 * absent method as a login or network failure.
 */
export interface DiscoveredMethod {
  name: string;
}

export interface DiscoverResult {
  methods: DiscoveredMethod[];
}

/** Identifies the host, connection and session a snapshot belongs to. */
export type CapabilityContext = string;

export interface CapabilitySnapshot {
  readonly context: CapabilityContext;
  readonly methods: ReadonlySet<string>;
}

export interface CapabilityLedger {
  /** Starts a discovery for `context`; the returned ticket identifies that request. */
  begin(context: CapabilityContext): number;
  /** Accepts a reply only if its ticket is still the newest and the context matches. */
  accept(ticket: number, context: CapabilityContext, result: DiscoverResult): boolean;
  /** Drops the snapshot. Pending replies from before this call are rejected. */
  invalidate(): void;
  supports(method: string): boolean | undefined;
  snapshot(): CapabilitySnapshot | null;
}

export function createCapabilityLedger(): CapabilityLedger {
  let current: CapabilitySnapshot | null = null;
  let newest = 0;
  let activeContext: CapabilityContext | null = null;

  return {
    begin(context) {
      newest += 1;
      activeContext = context;
      return newest;
    },
    accept(ticket, context, result) {
      if (ticket !== newest || context !== activeContext) return false;
      const names = new Set<string>();
      for (const method of result.methods ?? []) {
        if (typeof method?.name === 'string' && method.name.length > 0) names.add(method.name);
      }
      current = { context, methods: names };
      return true;
    },
    invalidate() {
      newest += 1;
      activeContext = null;
      current = null;
    },
    supports(method) {
      if (!current) return undefined;
      return current.methods.has(method);
    },
    snapshot() {
      return current;
    },
  };
}

/** Methods that only exist when Cloud features are compiled into the host. */
export const CLOUD_ACCOUNT_METHODS = ['cloud.status', 'cloud.restore'] as const;
export const CLOUD_SYNC_METHODS = ['sync.status'] as const;
