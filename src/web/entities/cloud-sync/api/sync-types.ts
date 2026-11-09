export interface SyncStatus {
  enabled: boolean;
  signedIn: boolean;
  syncing: boolean;
  pending: number;
  conflicts: string[];
  lastError?: string | null;
  lastSyncedAt?: number | null;
}

export interface SyncNowResult {
  pushed: number;
  pulled: number;
  applied: number;
  skipped: number;
  applicationDeferred: boolean;
  conflicts: string[];
}
