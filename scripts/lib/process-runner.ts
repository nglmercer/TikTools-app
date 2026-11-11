/**
 * Process execution shared by `scripts/task.ts` and `scripts/lib/test-runner.ts`.
 *
 * Importing this module never touches global process state: nothing is
 * registered until the launcher calls `forwardTerminationSignals`. Every
 * child started through a runner is tracked while it runs, and `cancel`
 * only ever signals those children, never unrelated processes. After a
 * cancellation no further command starts, and the cancellation surfaces as
 * `ProcessCancelled` carrying the conventional 130 (SIGINT) or 143
 * (SIGTERM) exit code instead of being reported as an ordinary failure.
 */

export type CancelSignal = 'SIGINT' | 'SIGTERM';

export function exitCodeForSignal(signal: CancelSignal): 130 | 143 {
  return signal === 'SIGINT' ? 130 : 143;
}

export class ProcessCancelled extends Error {
  readonly exitCode: 130 | 143;
  constructor(readonly signal: CancelSignal) {
    super(`cancelled by ${signal}`);
    this.name = 'ProcessCancelled';
    this.exitCode = exitCodeForSignal(signal);
  }
}

export type CaptureResult = { code: number | null; stdout: string; stderr: string };

export type ProcessRunner = {
  /** Run with inherited stdio; rejects when the command fails or the runner was cancelled. */
  run(cmd: string[]): Promise<void>;
  /** Run capturing stdout (and stderr unless `stderr: 'inherit'`). Never throws on a non-zero exit. */
  capture(cmd: string[], options?: { stderr?: 'inherit' | 'pipe' }): Promise<CaptureResult>;
  /** Signal every tracked child and refuse to start new commands. */
  cancel(signal: CancelSignal): void;
  readonly cancelledBy: CancelSignal | null;
  readonly activeChildren: number;
};

export type ProcessRunnerOptions = {
  cwd: string;
  env?: Record<string, string | undefined>;
  /** Injectable for tests; defaults to `Bun.spawn`. */
  spawn?: typeof Bun.spawn;
  /** Receives the `> command` line printed before each inherited-stdio run. */
  log?: (line: string) => void;
};

export function createProcessRunner(options: ProcessRunnerOptions): ProcessRunner {
  const spawn = options.spawn ?? Bun.spawn;
  const log = options.log ?? ((line: string) => console.log(line));
  const children = new Set<{ kill(signal?: NodeJS.Signals | number): void }>();
  let cancelledBy: CancelSignal | null = null;

  const assertNotCancelled = (): void => {
    if (cancelledBy) throw new ProcessCancelled(cancelledBy);
  };
  const assertCommand = (cmd: string[]): void => {
    if (cmd.length === 0) throw new Error('refusing to run an empty command');
  };

  const runner: ProcessRunner = {
    async run(cmd) {
      assertCommand(cmd);
      assertNotCancelled();
      log(`\n> ${cmd.join(' ')}`);
      const child = spawn(cmd, {
        cwd: options.cwd,
        stdin: 'inherit',
        stdout: 'inherit',
        stderr: 'inherit',
        env: options.env ?? process.env,
      });
      children.add(child);
      try {
        const code = await child.exited;
        // A cancelled child exits non-zero; report the cancellation, not a failure.
        assertNotCancelled();
        if (code !== 0) throw new Error(`command failed (exit ${code ?? 'signal'}): ${cmd.join(' ')}`);
      } finally {
        children.delete(child);
      }
    },

    async capture(cmd, captureOptions = {}) {
      assertCommand(cmd);
      assertNotCancelled();
      const stderrMode = captureOptions.stderr ?? 'pipe';
      const child = spawn(cmd, {
        cwd: options.cwd,
        stdout: 'pipe',
        stderr: stderrMode,
        env: options.env ?? process.env,
      });
      children.add(child);
      try {
        const [stdout, stderr, code] = await Promise.all([
          new Response(child.stdout as ReadableStream).text(),
          stderrMode === 'pipe' ? new Response(child.stderr as ReadableStream).text() : Promise.resolve(''),
          child.exited,
        ]);
        assertNotCancelled();
        return { code, stdout, stderr };
      } finally {
        children.delete(child);
      }
    },

    cancel(signal) {
      cancelledBy ??= signal;
      for (const child of [...children]) {
        try {
          child.kill(signal);
        } catch {
          // Already exited; nothing to stop.
        }
      }
    },

    get cancelledBy() {
      return cancelledBy;
    },
    get activeChildren() {
      return children.size;
    },
  };
  return runner;
}

/**
 * Cancel `runner` on SIGINT/SIGTERM. Only the launcher entry point should
 * call this; returns a function that removes the listeners.
 */
export function forwardTerminationSignals(
  runner: Pick<ProcessRunner, 'cancel'>,
  target: Pick<NodeJS.Process, 'on' | 'off'> = process,
): () => void {
  const handlers = new Map<CancelSignal, () => void>();
  for (const signal of ['SIGINT', 'SIGTERM'] as const) {
    const handler = (): void => runner.cancel(signal);
    handlers.set(signal, handler);
    target.on(signal, handler);
  }
  return () => {
    for (const [signal, handler] of handlers) target.off(signal, handler);
  };
}
