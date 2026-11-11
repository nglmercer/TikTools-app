import { readonly, ref, type InjectionKey, type Ref } from 'vue';

/**
 * Browser session lifecycle. A lost socket is transient (`reconnecting`); an
 * invalidated cookie (`expired`) can only be fixed by pairing again; a host
 * that never answers (`unavailable`) may still recover on its own.
 */
export type SessionState = 'ok' | 'reconnecting' | 'expired' | 'unavailable';
export type RecoveryOutcome = Exclude<SessionState, 'reconnecting'>;

/** Quiet retry budget before the dialog admits the host is unavailable. */
const DEFAULT_BACKOFF: readonly number[] = [0, 2_000, 5_000, 10_000, 20_000, 30_000];

export interface SessionRecovery {
  readonly state: Readonly<Ref<SessionState>>;
  readonly busy: Readonly<Ref<boolean>>;
  /** Entry point that starts a fresh pairing (pair page or join page). */
  readonly pairUrl: string;
  /** Feed an observed state. Auto-retries until the budget is spent. */
  report(state: SessionState): void;
  /** Explicit user action: try once and surface the outcome immediately. */
  retryNow(): Promise<void>;
  dispose(): void;
}

export const sessionRecoveryKey: InjectionKey<SessionRecovery> = Symbol('session-recovery');

/**
 * One recovery policy for both browser transports. The transport only reports
 * what it observed; this store owns backoff, so a dropped socket never turns
 * into a permanent "failed" screen while the cookie is still valid.
 */
export function createSessionRecovery(options: {
  pairUrl: string;
  retry: () => Promise<RecoveryOutcome>;
  backoff?: readonly number[];
}): SessionRecovery {
  const state = ref<SessionState>('ok');
  const busy = ref(false);
  const schedule = options.backoff ?? DEFAULT_BACKOFF;
  let attempt = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let disposed = false;

  function cancel(): void {
    if (timer !== undefined) {
      clearTimeout(timer);
      timer = undefined;
    }
  }

  function later(delay: number): void {
    cancel();
    timer = setTimeout(() => {
      timer = undefined;
      void run();
    }, delay);
  }

  async function run(): Promise<void> {
    if (disposed || busy.value) return;
    busy.value = true;
    let outcome: RecoveryOutcome;
    try {
      outcome = await options.retry();
    } catch {
      outcome = 'unavailable';
    }
    busy.value = false;
    if (disposed) return;
    if (outcome === 'ok') {
      state.value = 'ok';
      attempt = 0;
      cancel();
      return;
    }
    if (outcome === 'expired') {
      state.value = 'expired';
      cancel();
      return;
    }
    // Keep retrying quietly before showing the dialog.
    if (attempt >= schedule.length) {
      state.value = 'unavailable';
      cancel();
      return;
    }
    state.value = 'reconnecting';
    later(schedule[attempt++]!);
  }

  return {
    state: readonly(state),
    busy: readonly(busy),
    pairUrl: options.pairUrl,
    report(next) {
      if (disposed) return;
      if (next === 'ok') {
        state.value = 'ok';
        attempt = 0;
        cancel();
        return;
      }
      if (next === 'expired') {
        state.value = 'expired';
        cancel();
        return;
      }
      // A known-dead session stays that way until pairing or a successful
      // retry; a later socket close must not downgrade the message.
      if (state.value === 'expired') return;
      if (state.value === 'unavailable' && next === 'reconnecting') return;
      // An attempt already in flight owns the outcome and its retry budget.
      if (busy.value) return;
      state.value = 'reconnecting';
      attempt = 0;
      later(schedule[0] ?? 0);
    },
    async retryNow() {
      if (disposed) return;
      attempt = 0;
      cancel();
      state.value = 'reconnecting';
      await run();
      // A failed manual retry answers the click instead of starting a budget.
      if (state.value === 'reconnecting') {
        state.value = 'unavailable';
        cancel();
      }
    },
    dispose() {
      disposed = true;
      cancel();
    },
  };
}
