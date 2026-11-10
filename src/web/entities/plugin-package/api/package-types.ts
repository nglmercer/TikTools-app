export type InstallErrorCode =
  | 'already-installed'
  | 'not-installed'
  | 'not-found'
  | 'no-compatible-release'
  | 'download-failed'
  | 'digest-mismatch'
  | 'signature-required'
  | 'signature-invalid'
  | 'untrusted-publisher'
  | 'revoked'
  | 'invalid-package'
  | 'incompatible'
  | 'consent-required'
  | 'downgrade-blocked'
  | 'busy'
  | 'cancelled'
  | 'io-error'
  | 'unsupported';

export interface InstallError {
  code: InstallErrorCode;
  message: string;
  details?: unknown;
}

export type PackageSpecInput =
  | { kind: 'file'; path: string }
  | { kind: 'url'; url: string }
  | { kind: 'registry'; id: string; version?: string; registry?: string }
  | { kind: 'spec'; spec: string };

export type PackageOrigin =
  | { kind: 'registry'; registry: string; url: string }
  | { kind: 'url'; url: string }
  | { kind: 'file' }
  | { kind: 'builtin' }
  | { kind: 'development' }
  | { kind: 'adopted' };

export type SignatureStatus = 'verified' | 'unsigned' | 'untrusted';

export interface SignatureState {
  status: SignatureStatus;
  publisher?: string;
  keyId?: string;
  source?: string;
  reason?: string;
}

export type ChangeKind = 'install' | 'update' | 'reinstall' | 'downgrade';
export type SecurityModel = 'trusted' | 'isolated' | 'sandboxed';

export interface PermissionDiff {
  addedPermissions: string[];
  removedPermissions: string[];
  addedCapabilities: string[];
  removedCapabilities: string[];
}

export interface PluginAuthor {
  name: string;
  email?: string;
  url?: string;
}

export interface PackageMetadata {
  publisher?: string;
  authors?: PluginAuthor[];
  license?: string;
  homepage?: string;
  repository?: string;
  engine?: string;
  dependencies?: Record<string, string>;
  categories?: string[];
  changelog?: string;
}

export interface Incompatibility {
  kind: 'schema' | 'protocol' | 'abi' | 'host-version' | 'target' | 'runtime';
  version?: number;
  required?: string;
  host?: string;
  targets?: string[];
  runtime?: string;
}

export interface PackagePreview {
  id: string;
  name: string;
  version: string;
  description?: string;
  icon?: string;
  runtime: string;
  securityModel: SecurityModel;
  permissions: string[];
  capabilities: string[];
  size: number;
  digest: string;
  origin: PackageOrigin;
  signature: SignatureState;
  incompatibility?: Incompatibility;
  change: ChangeKind;
  installedVersion?: string;
  permissionDiff: PermissionDiff;
  requiresSideloadConsent: boolean;
  metadata: PackageMetadata;
  changelog?: string;
  consentToken: string;
}

export type JobKind = 'install' | 'update' | 'uninstall' | 'rollback';
export type JobStage = 'queued' | 'resolve' | 'download' | 'verify' | 'extract' | 'activate' | 'cleanup' | 'done';
export type JobStatus = 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled';

export interface JobSnapshot {
  id: string;
  kind: JobKind;
  pluginId?: string;
  spec?: string;
  fromVersion?: string;
  toVersion?: string;
  stage: JobStage;
  status: JobStatus;
  progress?: { done: number; total?: number };
  error?: InstallError;
  createdAt: number;
  updatedAt: number;
}

export type AutoUpdate = 'off' | 'notify' | 'patch' | 'minor';

export interface UpdatePolicy {
  channel: string;
  autoUpdate: AutoUpdate;
  pinned: boolean;
}

export interface RetainedVersion {
  version: string;
  retainedAt: number;
}

export interface InstallReceipt {
  id: string;
  version: string;
  origin: PackageOrigin;
  archiveDigest?: string;
  cloudArtifact?: { target: string; generation: string };
  signature: SignatureState;
  runtime: string;
  grantedPermissions: string[];
  grantedCapabilities: string[];
  installedAt: number;
  updatedAt: number;
  policy: UpdatePolicy;
  retained: RetainedVersion[];
}

export type AdvisorySeverity = 'low' | 'moderate' | 'high' | 'critical';

export interface Advisory {
  severity: AdvisorySeverity;
  message: string;
  url?: string;
}

/** Observation for the exact host artifact; omitted by older hosts means unknown. */
export type LiveReleaseStatus = 'unknown' | 'approved' | 'yanked' | 'key-revoked' | 'publisher-unapproved' | 'advisory' | 'incompatible';

export interface UpdateInfo {
  id: string;
  registry: string;
  current: string;
  latest?: string;
  kind?: 'patch' | 'minor' | 'major' | 'prerelease';
  channel: string;
  pinned: boolean;
  permissionDiff: PermissionDiff;
  advisory?: Advisory;
  changelog?: string;
  autoApplicable: boolean;
  currentStatus?: LiveReleaseStatus;
  latestStatus?: LiveReleaseStatus;
}

export interface UpdateReport {
  updates: UpdateInfo[];
  checkedAt: number;
  errors?: string[];
}

export interface AvailableRelease {
  version: string;
  channel: string;
  publishedAt: number;
  yanked: boolean;
  advisory?: Advisory;
  compatible: boolean;
  liveStatus?: LiveReleaseStatus;
  changelog?: string;
}

export interface PluginVersions {
  id: string;
  installed?: string;
  receipt?: InstallReceipt;
  retained: RetainedVersion[];
  registry?: string;
  available: AvailableRelease[];
  registryError?: string;
}

export interface RegistryStatus {
  name: string;
  url: string;
  enabled: boolean;
  priority: number;
  scopes: string[];
  builtin: boolean;
  verified: boolean;
  fingerprints: string[];
  description?: string;
  refreshedAt?: number;
  error?: string;
}

export interface RegistryDefaults {
  channel: string;
  autoUpdate: AutoUpdate;
  checkIntervalHours: number;
  allowSideload: boolean;
}

export interface CatalogEntry {
  id: string;
  name: string;
  publisher?: string;
  description?: string;
  icon?: string;
  categories?: string[];
  tags?: string[];
  latest: string;
  runtime: string;
  updatedAt: number;
}

export interface SearchEntry extends CatalogEntry {
  registry: string;
}

export interface SearchResults {
  entries: SearchEntry[];
  total: number;
  page: number;
  pages: number;
  errors?: string[];
}

export interface RegistryRelease {
  version: string;
  channel: string;
  publishedAt: number;
  yanked: boolean;
  advisory?: Advisory;
  runtime: string;
  permissions: string[];
  capabilities: string[];
  changelog?: string;
}

export interface RegistryPackageDocument {
  id: string;
  name: string;
  publisher?: string;
  description?: string;
  icon?: string;
  categories?: string[];
  tags?: string[];
  homepage?: string;
  repository?: string;
  license?: string;
  screenshots?: string[];
  'dist-tags'?: Record<string, string>;
  releases: RegistryRelease[];
}

export interface RegistryPackageView {
  registry: string;
  document: RegistryPackageDocument;
  liveStatuses?: Record<string, LiveReleaseStatus>;
}
