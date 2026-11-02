import type { JobSnapshot, PackageOrigin } from '../api/package-types.ts';

const UNITS = ['B', 'KB', 'MB', 'GB'] as const;

export function formatBytes(bytes: number): string {
  let value = Math.max(0, bytes);
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${unit === 0 ? value : value.toFixed(1)} ${UNITS[unit]}`;
}

export function shortDigest(digest: string): string {
  const hex = digest.replace(/^sha256:/, '');
  return hex.length > 16 ? `${hex.slice(0, 8)}…${hex.slice(-8)}` : hex;
}

export function originLabelKey(origin: PackageOrigin): string {
  return `packages.origin.${origin.kind}`;
}

export function originDetail(origin: PackageOrigin): string {
  if (origin.kind === 'registry') return origin.registry;
  if (origin.kind === 'url') return origin.url;
  return '';
}

export function jobPercent(job: JobSnapshot): number | null {
  const total = job.progress?.total;
  if (!job.progress || !total) return null;
  return Math.min(100, Math.round((job.progress.done / total) * 100));
}

export function isActiveJob(job: JobSnapshot): boolean {
  return job.status === 'queued' || job.status === 'running';
}

export function remoteSelection(registry: string, id: string): string {
  return `registry:${registry}/${id}`;
}

export function parseRemoteSelection(value: string | null): { registry: string; id: string } | null {
  if (!value?.startsWith('registry:')) return null;
  const rest = value.slice('registry:'.length);
  const slash = rest.indexOf('/');
  if (slash <= 0 || slash === rest.length - 1) return null;
  return { registry: rest.slice(0, slash), id: rest.slice(slash + 1) };
}
