import { describe, expect, test } from 'bun:test';
import { join } from 'node:path';

const root = join(import.meta.dir, '..', '..', '..');

function task(...args: string[]): {
  code: number;
  output: string
} {
  const result = Bun.spawnSync({
    cmd: [process.execPath, 'scripts/task.ts', ...args],
    cwd: root,
    stdout: 'pipe',
    stderr: 'pipe',
  });
  return {
    code: result.exitCode ?? 1,
    output: `${result.stdout.toString()}\n${result.stderr.toString()}`,
  };
}

describe('task.ts argument validation', () => {
  test('unknown command prints usage with exit 2', () => {
    const result = task('bogus-command');
    expect(result.code).toBe(2);
    expect(result.output).toContain('Usage:');
  });

  test('missing command prints usage with exit 2', () => {
    const result = task();
    expect(result.code).toBe(2);
    expect(result.output).toContain('Usage:');
  });

  test('public gates reject unexpected arguments instead of ignoring them', () => {
    for (const command of ['build', 'lint', 'ci', 'start', 'dev']) {
      const result = task(command, '--unexpected-flag');
      expect(result.code).toBe(1);
      expect(result.output).toContain('takes no arguments');
    }
  });

  test('unknown version subcommand fails loudly', () => {
    const result = task('version', 'bogus');
    expect(result.code).toBe(1);
    expect(result.output).toContain('unknown version subcommand');
  });

  test('rust without cargo arguments fails loudly', () => {
    const result = task('rust');
    expect(result.code).toBe(1);
    expect(result.output).toContain('needs cargo arguments');
  });
});
