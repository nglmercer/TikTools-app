/** Profiler parsing and missing-process regressions;
 * no running host required. */
import { describe, expect, test } from 'bun:test';
import {
  cpuTicks,
  descendants,
  healthDefault,
  hostActivityDefault,
  runSampling,
  windowsSnapshot,
  type ProcessSample,
  type ProcInfo,
  type SampleOptions,
} from '../../../scripts/profile-live-memory.ts';

function proc(overrides: Partial<ProcInfo> = {}): ProcInfo {
  return { ppid: 0, name: 'host', created: '2026-01-01T00:00:00', rss: 0, threads: 1, cpu: 0, ...overrides };
}

function splitCsv(line: string): string[] {
  const cells: string[] = [];
  let current = '';
  let quoted = false;
  for (let index = 0; index < line.length; index += 1) {
    const char = line.charAt(index);
    if (quoted) {
      if (char === '"') {
        if (line[index + 1] === '"') {
          current += '"';
          index += 1;
        } else {
          quoted = false;
        }
      } else {
        current += char;
      }
    } else if (char === '"') {
      quoted = true;
    } else if (char === ',') {
      cells.push(current);
      current = '';
    } else {
      current += char;
    }
  }
  cells.push(current);
  return cells;
}

describe('profile-live-memory', () => {
  test('windows snapshot ignores missing hosts and unrelated trees', () => {
    const processes = { 1: proc({ ppid: 0, created: '10' }), 2: proc({ ppid: 0, created: '11' }) };
    expect(windowsSnapshot(1, processes)).toEqual({ tree: { 1: processes[1] }, truncated: false });
    expect(windowsSnapshot(99, processes)).toEqual({ tree: {}, truncated: false });
  });

  test('windows traversal follows children but excludes reused parent PIDs', () => {
    const processes = {
      1: proc({ ppid: 0, created: '10' }),
      2: proc({ ppid: 1, created: '11' }),
      3: proc({ ppid: 2, created: '12' }),
      4: proc({ ppid: 1, created: '09' }),
    };
    const { tree, truncated } = windowsSnapshot(1, processes);
    expect(new Set(Object.keys(tree).map(Number))).toEqual(new Set([1, 2, 3]));
    expect(truncated).toBe(false);
  });

  test('linux traversal deduplicates shared children', () => {
    const samples = new Map<number, ProcessSample>([
      [1, { rss: 10, name: 'host', children: [2, 3] }],
      [2, { rss: 20, name: 'webkit', children: [3] }],
      [3, { rss: 30, name: 'plugin', children: [] }],
    ]);
    const sample = (pid: number) => samples.get(pid) ?? null;
    expect(descendants(1, sample, () => true)).toEqual([10, 30, 20, 0, false]);
  });

  test('reused linux child is not attributed', () => {
    const samples = new Map<number, ProcessSample>([
      [1, { rss: 10, name: 'host', children: [2] }],
      [2, { rss: 20, name: 'webkit', children: [] }],
    ]);
    const sample = (pid: number) => samples.get(pid) ?? null;
    expect(descendants(1, sample, () => false)).toEqual([10, 0, 0, 0, false]);
  });

  test('missing health CLI is unavailable, not a failure', () => {
    expect(healthDefault('/nonexistent/tiktools-cli-for-tests')).toEqual({ counts: {}, state: 'unavailable' });
  });

  test('csv header matches row shape', async () => {
    const lines: string[] = [];
    const options: SampleOptions = {
      pid: process.pid,
      cli: '/nonexistent/tiktools-cli-for-tests',
      samples: 1,
      interval: 0.1,
      label: 'test',
      commit: 'test',
      phase: 'load',
    };
    await runSampling(options, {
      health: () => ({ counts: {}, state: 'unavailable' }),
      sleep: async () => {},
      write: (line: string) => lines.push(line),
    });
    expect(lines).toHaveLength(2);
    const [header = '', row = ''] = lines;
    expect(splitCsv(header)).toHaveLength(splitCsv(row).length);
    expect(header).toContain('process_tree_rss_sum_kib');
  });

  test('cpu ticks accept parentheses and spaces in process names', () => {
    const fields = ['R', ...Array<string>(10).fill('0'), '23', '17', ...Array<string>(8).fill('0')];
    expect(cpuTicks(`123 (name (with) spaces) ${fields.join(' ')}`)).toBe(40);
  });

  test('missing host is unknown, not zero rss', () => {
    expect(descendants(123456789, () => null)[0]).toBeNull();
  });

  const isLinux = process.platform === 'linux';
  (isLinux ? test : test.skip)('host threads and cpu agree with proc', () => {
    const [threads, ticks] = hostActivityDefault(process.pid);
    expect(threads ?? 0).toBeGreaterThanOrEqual(1);
    expect(ticks ?? 0).toBeGreaterThanOrEqual(0);
    expect(descendants(process.pid)[0] ?? 0).toBeGreaterThan(0);
  });
});
