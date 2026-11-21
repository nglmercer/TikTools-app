export type AccountState = 'unconfigured' | 'signed-out' | 'signing-in' | 'signed-in';

export type AccountPersistence = 'keychain' | 'memory-only';

export interface AccountStatus {
  origin: string | null;
  state: AccountState;
  loginOperationId?: string | null;
  accountId?: string | null;
  email?: string | null;
  displayName?: string | null;
  persistence: AccountPersistence;
  loginCode?: LoginCodeStatus | null;
  error?: string | null;
}

export interface LoginCodeStatus {
  email: string;
  expiresAt: number;
  delivered: number;
  requiresOauth: boolean;
}

export interface CloudLoginUrlResult {
  browserUrl: string;
}

/** Plan ceilings the backend enforces for the signed-in account (nonsecret). */
export interface AccountPlanLimits {
  maxRecords: number;
  maxBytes: number;
  maxAssets: number;
  maxAssetsBytes: number;
  maxOverlays: number;
  maxOverlayBytes: number;
  maxBackups: number;
  maxBackupBytes: number;
}

/** Storage the signed-in account uses now, counted the way the quota checks count it. */
export interface AccountPlanUsage {
  records: number;
  recordBytes: number;
  assets: number;
  assetBytes: number;
  overlays: number;
  overlayBytes: number;
  backups: number;
  backupBytes: number;
}

export interface AccountPlan {
  plan: string;
  limits: AccountPlanLimits;
  usage: AccountPlanUsage;
}

/** Opaque, versioned portable copy of everything the cloud owns. */
export interface BackupManifest {
  schemaVersion: number;
  createdAt: string;
  [key: string]: unknown;
}

/** A backup stored in the cloud for the signed-in account. */
export interface StoredBackup {
  id: string;
  name: string;
  createdAt: string;
  sizeBytes: number;
  workspaceSync: number;
  library: number;
  mediaLibrary: number;
  assets: number;
}

/** Counts returned by a dry-run restore before anything is changed. */
export interface BackupSummary {
  mode: 'replace' | 'merge';
  dryRun: boolean;
  applied: boolean;
  legacySync: number;
  workspaceSync: number;
  library: number;
  mediaLibrary: number;
  assets: number;
}
