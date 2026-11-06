#!/usr/bin/env bun
/**
 * Opt-in process-tree RSS sampler (Bun port of the retired
 * `scripts/profile-live-memory.py`; the Windows-only wrapper
 * `profile-live-memory-windows.py` is consolidated into this file).
 * No third-party Python packages are required: Linux reads /proc
 * directly and Windows samples through the built-in CIM cmdlets.
 * RSS sums include shared pages; they are not private/unique memory.
 * Never reads process arguments or environment variables.
 */

import { createHash } from 'node:crypto';
import { readdirSync, readFileSync } from 'node:fs';

export const IS_WINDOWS = process.platform === 'win32';

export type ProcInfo = {
  ppid: number;
  name: string;
  created: string;
  rss: number;
  threads: number;
  cpu: number;
};

export type ProcessSample = { rss: number; name: string; children: number[] } | null;

export const CSV_HEADER = [
  'sample',
  'host_rss_kib',
  'plugin_rss_kib',
  'webview_rss_kib',
  'other_child_rss_kib',
  'children_truncated',
  'health',
  'analytics_pending',
  'analytics_failures',
  'analytics_overload',
  'plugin_workers',
  'plugin_outstanding_calls',
  'plugin_dropped_events',
  'automation_tasks',
  'points_hot',
  'points_hot_capacity',
  'points_writer_queued',
  'points_write_errors',
  'points_writer_overload',
  'ingress_queued',
  'ingress_accepted',
  'ingress_dropped_data',
  'ingress_dropped_critical',
  'ingress_lagged',
  'dedup_duplicates',
  'automation_admitted',
  'automation_rejected',
  'automation_actions_failed',
  'processor_overloaded',
  'memory_host_rss_bytes',
  'label',
  'phase',
  'elapsed_seconds',
  'host_threads',
  'host_cpu_percent',
  'connection_generation',
  'connection_state',
  'build_commit',
  'host_pid',
  'process_tree_rss_sum_kib',
  'tree_processes',
  'webview_processes',
  'tree_threads',
  'memory_kind',
  'desktop_diagnostics',
];

export const MEMORY_KIND = 'RSS_sum_shared_pages_included';
const TREE_LIMIT = 256;

/** Associate descendants by PID and creation time, never by name alone. */
export function windowsSnapshot(
  pid: number,
  processes: Record<number, ProcInfo>,
): { tree: Record<number, ProcInfo>; truncated: boolean } {
  const host = processes[pid];
  if (!host) return { tree: {}, truncated: false };
  const pending = [pid];
  const tree: Record<number, ProcInfo> = {};
  while (pending.length > 0 && Object.keys(tree).length < TREE_LIMIT) {
    const current = pending.pop() as number;
    if (current in tree || !(current in processes)) continue;
    const parent = processes[current] as ProcInfo;
    tree[current] = parent;
    for (const [childText, info] of Object.entries(processes)) {
      const child = Number(childText);
      if (info.ppid === current && child !== current && info.created >= parent.created) {
        pending.push(child);
      }
    }
  }
  return { tree, truncated: pending.length > 0 };
}

export function processIdentityDefault(pid: number): string | null {
  if (IS_WINDOWS) return windowsIdentity(pid);
  try {
    // starttime is field 22; comm may contain spaces and parentheses, so
    // fields after the final closing parenthesis start at field 3.
    const fields = readFileSyncText(`/proc/${pid}/stat`).split(')').pop()?.trim().split(/\s+/);
    return fields && fields.length > 19 ? (fields[19] as string) : null;
  } catch {
    return null;
  }
}

function readFileSyncText(path: string): string {
  return readFileSync(path, 'utf8');
}

function windowsIdentity(pid: number): string | null {
  const rows = queryWindowsProcesses(`-Filter "ProcessId = ${pid}"`);
  const info = rows.find((row) => row.pid === pid);
  return info ? info.created : null;
}

type WindowsRow = {
  pid: number;
  ppid: number;
  name: string;
  created: string;
  rss: number;
  threads: number;
  cpu: number;
};

function powershell(): string | null {
  for (const candidate of ['pwsh', 'powershell']) {
    const probe = Bun.spawnSync({ cmd: [candidate, '-NoProfile', '-Command', '$PSVersionTable.PSVersion.Major'], stdout: 'pipe', stderr: 'ignore' });
    if (probe.success) return candidate;
  }
  return null;
}

function queryWindowsProcesses(filter = ''): WindowsRow[] {
  const shell = powershell();
  if (!shell) return [];
  const select =
    "Select-Object @{N='Pid';E={$_.ProcessId}},@{N='Ppid';E={$_.ParentProcessId}},@{N='Name';E={$_.Name}},@{N='Created';E={$_.CreationDate.ToString('o')}},@{N='Rss';E={$_.WorkingSetSize}},@{N='Threads';E={$_.ThreadCount}},@{N='UserTime';E={$_.UserModeTime}},@{N='KernelTime';E={$_.KernelModeTime}}";
  const result = Bun.spawnSync({
    cmd: [shell, '-NoProfile', '-NonInteractive', '-Command', `Get-CimInstance Win32_Process ${filter} | ${select} | ConvertTo-Json -Depth 3 -Compress`],
    stdout: 'pipe',
    stderr: 'ignore',
    timeout: 15000,
  });
  if (!result.success) return [];
  const text = result.stdout.toString().trim();
  if (!text) return [];
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return [];
  }
  const rows = Array.isArray(parsed) ? parsed : [parsed];
  const out: WindowsRow[] = [];
  for (const row of rows) {
    if (!row || typeof row !== 'object') continue;
    const record = row as Record<string, unknown>;
    if (typeof record['Pid'] !== 'number') continue;
    const user = typeof record['UserTime'] === 'number' ? (record['UserTime'] as number) : 0;
    const kernel = typeof record['KernelTime'] === 'number' ? (record['KernelTime'] as number) : 0;
    out.push({
      pid: record['Pid'] as number,
      ppid: typeof record['Ppid'] === 'number' ? (record['Ppid'] as number) : 0,
      name: String(record['Name'] ?? '').toLowerCase(),
      created: String(record['Created'] ?? ''),
      rss: Math.floor(Number(record['Rss'] ?? 0) / 1024),
      threads: typeof record['Threads'] === 'number' ? (record['Threads'] as number) : 0,
      cpu: (user + kernel) / 1e7,
    });
  }
  return out;
}

export function windowsProcessesDefault(): Record<number, ProcInfo> {
  const processes: Record<number, ProcInfo> = {};
  for (const row of queryWindowsProcesses()) {
    processes[row.pid] = {
      ppid: row.ppid,
      name: row.name,
      created: row.created,
      rss: row.rss,
      threads: row.threads,
      cpu: row.cpu,
    };
  }
  return processes;
}

export function processSampleDefault(pid: number): ProcessSample {
  try {
    const status = readFileSyncText(`/proc/${pid}/status`);
    const rssLine = status.split('\n').find((line) => line.startsWith('VmRSS:'));
    if (!rssLine) return null;
    const rss = Number(rssLine.split(/\s+/)[1]);
    const name = readFileSyncText(`/proc/${pid}/comm`).trim().toLowerCase();
    const children: number[] = [];
    let tasks: string[] = [];
    try {
      tasks = readdirSync(`/proc/${pid}/task`);
    } catch {
      return { rss, name, children };
    }
    for (const task of tasks) {
      try {
        const text = readFileSync(`/proc/${pid}/task/${task}/children`, 'utf8');
        for (const part of text.split(/\s+/)) {
          if (part) children.push(Number(part));
        }
      } catch {
        continue;
      }
    }
    if (!Number.isFinite(rss)) return null;
    return { rss, name, children };
  } catch {
    return null;
  }
}

export function belongsToParentDefault(child: number, parent: number): boolean {
  try {
    const childFields = readFileSyncText(`/proc/${child}/stat`).split(')').pop()?.trim().split(/\s+/);
    const parentFields = readFileSyncText(`/proc/${parent}/stat`).split(')').pop()?.trim().split(/\s+/);
    if (!childFields || !parentFields) return false;
    return Number(childFields[1]) === parent && Number(childFields[19]) >= Number(parentFields[19]);
  } catch {
    return false;
  }
}

/** utime + stime in clock ticks; comm may contain spaces and parentheses. */
export function cpuTicks(stat: string): number {
  const fields = (stat.split(')').pop() ?? '').trim().split(/\s+/);
  return Number(fields[11]) + Number(fields[12]);
}

export function hostActivityDefault(pid: number): [number | null, number | null] {
  try {
    const status = readFileSyncText(`/proc/${pid}/status`);
    const threadsLine = status.split('\n').find((line) => line.startsWith('Threads:'));
    if (!threadsLine) return [null, null];
    const threads = Number(threadsLine.split(/\s+/)[1]);
    const ticks = cpuTicks(readFileSyncText(`/proc/${pid}/stat`));
    if (!Number.isFinite(threads) || !Number.isFinite(ticks)) return [null, null];
    return [threads, ticks];
  } catch {
    return [null, null];
  }
}

export function treeActivity(
  pid: number,
  sample: (pid: number) => ProcessSample = processSampleDefault,
  belongs: (child: number, parent: number) => boolean = belongsToParentDefault,
  activity: (pid: number) => [number | null, number | null] = hostActivityDefault,
): [number, number, number] {
  const pending = [pid];
  const seen = new Set<number>();
  let processes = 0;
  let webviews = 0;
  let threads = 0;
  while (pending.length > 0 && seen.size < TREE_LIMIT) {
    const child = pending.pop() as number;
    if (seen.has(child)) continue;
    seen.add(child);
    const current = sample(child);
    if (!current) continue;
    processes += 1;
    if (current.name.includes('webkit') || current.name.includes('webview')) webviews += 1;
    threads += activity(child)[0] ?? 0;
    for (const childPid of current.children) {
      if (belongs(childPid, child)) pending.push(childPid);
    }
  }
  return [processes, webviews, threads];
}

export function descendants(
  pid: number,
  sample: (pid: number) => ProcessSample = processSampleDefault,
  belongs: (child: number, parent: number) => boolean = belongsToParentDefault,
): [number | null, number, number, number, boolean] {
  const pending = [pid];
  const seen = new Set<number>();
  let host: number | null = null;
  let plugins = 0;
  let webviews = 0;
  let other = 0;
  while (pending.length > 0 && seen.size < TREE_LIMIT) {
    const child = pending.pop() as number;
    if (seen.has(child)) continue;
    seen.add(child);
    const current = sample(child);
    if (!current) continue;
    const { rss, name, children } = current;
    for (const childPid of children) {
      if (belongs(childPid, child)) pending.push(childPid);
    }
    if (child === pid) {
      host = rss;
    } else if (name.includes('webkit') || name.includes('webview')) {
      webviews += rss;
    } else if (name.includes('plugin') || name.includes('sonicboom')) {
      plugins += rss;
    } else {
      other += rss;
    }
  }
  return [host, plugins, webviews, other, pending.length > 0];
}

export function healthDefault(cli: string): { counts: Record<string, unknown>; state: string } {
  try {
    const result = Bun.spawnSync({
      cmd: [cli, 'rpc', 'system.health', '{}'],
      stdout: 'pipe',
      stderr: 'ignore',
      timeout: 5000,
    });
    if (!result.success) return { counts: {}, state: 'unavailable' };
    const value: unknown = JSON.parse(result.stdout.toString());
    const counts =
      value && typeof value === 'object' && !Array.isArray(value)
        ? ((value as Record<string, unknown>)['result'] ?? value)
        : null;
    if (counts && typeof counts === 'object' && !Array.isArray(counts)) {
      return { counts: counts as Record<string, unknown>, state: 'ok' };
    }
    return { counts: {}, state: 'unavailable' };
  } catch {
    return { counts: {}, state: 'unavailable' };
  }
}

export type SampleOptions = {
  pid: number;
  cli: string;
  samples: number;
  interval: number;
  label: string;
  commit: string;
  phase: 'idle' | 'load' | 'cleanup';
};

export type SamplingDeps = {
  processIdentity?: (pid: number) => string | null;
  health?: (cli: string) => { counts: Record<string, unknown>; state: string };
  windowsProcesses?: () => Record<number, ProcInfo>;
  sample?: (pid: number) => ProcessSample;
  belongs?: (child: number, parent: number) => boolean;
  hostActivity?: (pid: number) => [number | null, number | null];
  clockTicks?: () => number;
  now?: () => number;
  sleep?: (ms: number) => Promise<void>;
  write?: (line: string) => void;
};

function record(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function cell(value: unknown): string {
  if (value === null || value === undefined) return '';
  if (typeof value === 'boolean') return value ? 'True' : 'False';
  const text = String(value);
  return text.includes(',') || text.includes('"') || text.includes('\n') || text.includes('\r')
    ? `"${text.replaceAll('"', '""')}"`
    : text;
}

export function csvLine(values: unknown[]): string {
  return values.map(cell).join(',');
}

function clockTicksDefault(): number {
  if (IS_WINDOWS) return 1;
  try {
    const result = Bun.spawnSync({ cmd: ['getconf', 'CLK_TCK'], stdout: 'pipe', stderr: 'ignore' });
    const parsed = Number(result.stdout.toString().trim());
    return Number.isFinite(parsed) && parsed > 0 ? parsed : 100;
  } catch {
    return 100;
  }
}

export async function runSampling(options: SampleOptions, deps: SamplingDeps = {}): Promise<void> {
  const processIdentity = deps.processIdentity ?? processIdentityDefault;
  const health = deps.health ?? healthDefault;
  const windowsProcesses = deps.windowsProcesses ?? windowsProcessesDefault;
  const sample = deps.sample ?? processSampleDefault;
  const belongs = deps.belongs ?? belongsToParentDefault;
  const hostActivity = deps.hostActivity ?? hostActivityDefault;
  const clockTicks = (deps.clockTicks ?? clockTicksDefault)();
  const now = deps.now ?? (() => performance.now() / 1000);
  const sleep = deps.sleep ?? ((ms: number) => Bun.sleep(ms));
  const write = deps.write ?? ((line: string) => console.log(line));

  const { pid, cli, samples, interval, label, commit, phase } = options;
  const identity = processIdentity(pid);
  const began = now();
  write(csvLine(CSV_HEADER));
  let previousTicks: number | null = null;
  let previousAt: number | null = null;
  for (let index = 0; index < samples; index += 1) {
    let { counts, state } = health(cli);
    if (counts['pid'] !== undefined && counts['pid'] !== pid) {
      counts = {};
      state = 'pid_mismatch';
    }
    const at = now();
    const sameHost = identity !== null && identity !== undefined && processIdentity(pid) === identity;
    if (!sameHost) {
      counts = {};
      state = 'host_exited_or_reused';
    }
    let rss: [number | null, number, number, number, boolean];
    let activity: [number, number, number];
    let threads: number | null;
    let ticks: number | null;
    if (IS_WINDOWS) {
      const { tree, truncated } = sameHost ? windowsSnapshot(pid, windowsProcesses()) : { tree: {}, truncated: false };
      let host: ProcInfo | null = tree[pid] ?? null;
      if (host && host.created !== identity) {
        for (const key of Object.keys(tree)) delete tree[Number(key)];
        host = null;
        counts = {};
        state = 'host_exited_or_reused';
      }
      threads = host ? host.threads : null;
      ticks = host ? host.cpu : null;
      const lanes: [number | null, number, number, number, boolean] = [host ? host.rss : null, 0, 0, 0, truncated];
      for (const [childText, info] of Object.entries(tree)) {
        if (Number(childText) === pid) continue;
        const lane =
          info.name.includes('webview') || info.name.includes('webkit')
            ? 2
            : info.name.includes('plugin')
              ? 1
              : 3;
        (lanes[lane] as number) += info.rss;
      }
      rss = lanes;
      activity = [
        Object.keys(tree).length,
        Object.values(tree).filter((info) => info.name.includes('webview') || info.name.includes('webkit')).length,
        Object.values(tree).reduce((total, info) => total + info.threads, 0),
      ];
    } else {
      [threads, ticks] = sameHost ? hostActivity(pid) : [null, null];
      rss = sameHost ? descendants(pid, sample, belongs) : [null, 0, 0, 0, false];
      activity = sameHost ? treeActivity(pid, sample, belongs, hostActivity) : [0, 0, 0];
    }
    let cpu: number | null = null;
    if (ticks !== null && previousTicks !== null && previousAt !== null && ticks >= previousTicks) {
      cpu = Math.round((100 * (ticks - previousTicks) * 1000) / clockTicks / ((at - previousAt) * 1000)) / 1000;
    }
    previousTicks = ticks;
    previousAt = at;
    const connection = record(counts['liveConnection']);
    const analytics = record(counts['analytics']);
    const plugins = record(counts['plugins']);
    const points = record(counts['points']);
    const ingress = record(counts['liveIngress']);
    const dedup = record(counts['liveDedup']);
    const automation = record(counts['automation']);
    const processors = record(counts['processorStats']);
    const memory = record(counts['memory']);
    const desktop = counts['desktop'];
    const treeSum = rss[0] === null || rss[0] === undefined ? null : (rss[0] as number) + rss[1] + rss[2] + rss[3];
    write(
      csvLine([
        index,
        ...rss,
        state,
        analytics['pending'],
        analytics['failures'],
        analytics['overload'],
        plugins['eventWorkers'],
        plugins['outstandingCalls'],
        plugins['droppedEvents'],
        counts['automationActiveTasks'],
        points['hotSize'],
        points['hotCapacity'],
        points['writerQueued'],
        points['writeErrors'],
        points['writerOverload'],
        ingress['queued'],
        ingress['accepted'],
        ingress['droppedData'],
        ingress['droppedCritical'],
        ingress['broadcastLagged'],
        dedup['duplicates'],
        automation['admitted'],
        automation['rejected'],
        automation['actionsFailed'],
        processors['overloaded'],
        memory['hostRssBytes'],
        label,
        phase,
        Math.round((at - began) * 1000) / 1000,
        threads,
        cpu,
        connection['generation'],
        connection['state'],
        commit,
        pid,
        treeSum,
        ...activity,
        MEMORY_KIND,
        desktop === undefined || desktop === null ? '' : JSON.stringify(desktop),
      ]),
    );
    if (index + 1 < samples) await sleep(interval * 1000);
  }
}

export function parseArgs(argv: string[]): SampleOptions {
  const get = (flag: string): string | undefined => {
    const index = argv.indexOf(flag);
    return index >= 0 ? argv[index + 1] : undefined;
  };
  const has = (flag: string): boolean => argv.includes(flag);
  if (has('--help') || has('-h')) {
    console.log(
      'Opt-in process-tree RSS sampler. RSS sums include shared pages; they are not private/unique memory.\n' +
        'Options: --pid <PID> (required) [--cli <path>] [--samples N] [--interval S]\n' +
        '         [--label <text>] [--commit <sha>] [--phase idle|load|cleanup]',
    );
    process.exit(0);
  }
  const fail = (message: string): never => {
    console.error(`profile-live-memory: ${message}`);
    process.exit(2);
  };
  const pidRaw = get('--pid');
  if (pidRaw === undefined) fail('--pid <running host PID> is required');
  const pid = Number(pidRaw);
  const samples = Number(get('--samples') ?? '120');
  const interval = Number(get('--interval') ?? '1.0');
  const label = get('--label') ?? 'unlabelled';
  const commit = get('--commit') ?? 'unknown';
  const phase = (get('--phase') ?? 'load') as SampleOptions['phase'];
  if (!Number.isFinite(pid) || pid <= 0 || !Number.isInteger(pid)) fail('use a positive PID');
  if (!Number.isInteger(samples) || samples < 1 || samples > 3600) fail('use 1..3600 samples');
  if (!Number.isFinite(interval) || interval < 0.1 || interval > 60) fail('use a 0.1..60 second interval');
  if (label.length > 64 || [...label].some((char) => char < ' ')) {
    fail('label must be at most 64 printable characters');
  }
  if (phase !== 'idle' && phase !== 'load' && phase !== 'cleanup') fail('phase must be idle, load, or cleanup');
  return { pid, cli: get('--cli') ?? 'target/debug/tiktools', samples, interval, label, commit, phase };
}

export function sha256Hex(data: Uint8Array | string): string {
  return createHash('sha256').update(data).digest('hex');
}

if (import.meta.main) {
  await runSampling(parseArgs(process.argv.slice(2)));
}
