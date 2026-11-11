import { describe, expect, test } from 'bun:test';
import { ProcessCancelled, createProcessRunner, exitCodeForSignal, forwardTerminationSignals } from './process-runner.ts';

const bun = process.execPath;
const quiet = { cwd: process.cwd(), log: () => {} };
/** An ephemeral child that stays alive until it is signalled. */
const sleeper = [bun, '-e', 'setTimeout(() => {}, 60000)'];

async function waitFor(condition: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 200 && !condition(); attempt += 1) await Bun.sleep(10);
  expect(condition()).toBe(true);
}

describe('process runner', () => {
  test('runs a command and fails with its exit code', async () => {
    const runner = createProcessRunner({ ...quiet, env: process.env });
    const ok = [bun, '-e', 'void 0'];
    await runner.run(ok);
    await expect(runner.run([bun, '-e', 'process.exit(3)'])).rejects.toThrow(/exit 3/);
    expect(runner.activeChildren).toBe(0);
  });

  test('refuses an empty command', async () => {
    await expect(createProcessRunner(quiet).run([])).rejects.toThrow(/empty command/);
  });

  test('capture returns stdout and the exit code without throwing', async () => {
    const runner = createProcessRunner(quiet);
    const result = await runner.capture([bun, '-e', 'console.log("listed"); process.exit(4)']);
    expect(result.code).toBe(4);
    expect(result.stdout.trim()).toBe('listed');
    expect(runner.activeChildren).toBe(0);
  });

  test('cancelling stops the tracked child, reports 130/143 and blocks the next command', async () => {
    for (const signal of ['SIGINT', 'SIGTERM'] as const) {
      const runner = createProcessRunner(quiet);
      const running = runner.run(sleeper);
      const outcome = running.then(
        () => null,
        (error: unknown) => error,
      );
      await waitFor(() => runner.activeChildren === 1);

      runner.cancel(signal);
      const error = await outcome;
      expect(error).toBeInstanceOf(ProcessCancelled);
      expect((error as ProcessCancelled).exitCode).toBe(exitCodeForSignal(signal));
      expect(runner.activeChildren).toBe(0);

      // A queued follow-up command must never start.
      await expect(runner.run([bun, '-e', 'void 0'])).rejects.toBeInstanceOf(ProcessCancelled);
      await expect(runner.capture([bun, '-e', 'void 0'])).rejects.toBeInstanceOf(ProcessCancelled);
    }
  });

  test('cancel only signals children it started', async () => {
    const unrelated = Bun.spawn(sleeper, { stdout: 'ignore', stderr: 'ignore' });
    try {
      const runner = createProcessRunner(quiet);
      runner.cancel('SIGTERM');
      await Bun.sleep(50);
      expect(unrelated.exitCode).toBeNull();
    } finally {
      unrelated.kill();
      await unrelated.exited;
    }
  });
});

describe('signal forwarding', () => {
  test('importing the module registers nothing; forwarding registers and removes handlers', () => {
    const before = [process.listenerCount('SIGINT'), process.listenerCount('SIGTERM')];
    const registered: Array<[string, () => void]> = [];
    const removed: string[] = [];
    const cancelled: string[] = [];
    const stop = forwardTerminationSignals(
      { cancel: (signal) => void cancelled.push(signal) },
      {
        on: (signal: string, handler: () => void) => void registered.push([signal, handler]),
        off: (signal: string) => void removed.push(signal),
      } as never,
    );
    expect([process.listenerCount('SIGINT'), process.listenerCount('SIGTERM')]).toEqual(before);
    for (const [, handler] of registered) handler();
    expect(cancelled).toEqual(['SIGINT', 'SIGTERM']);
    stop();
    expect(removed).toEqual(['SIGINT', 'SIGTERM']);
  });
});
