export type UpdateChannel = 'stable' | 'beta';
export type UpdatePhase = 'idle' | 'checking' | 'upToDate' | 'available' | 'downloading' | 'verifying' | 'readyToInstall' | 'installing' | 'restartRequired' | 'completed' | 'failed';

export interface UpdateStatus {
  currentVersion: string;
  availableVersion: string | null;
  channel: UpdateChannel;
  state: UpdatePhase;
  installationType: string;
  installSupported: boolean;
  checkSupported: boolean;
  downloadedBytes: number;
  totalBytes: number | null;
  progress: number | null;
  releaseNotes: string | null;
  lastChecked: number | null;
  error: { code: string; message: string } | null;
}

export interface UpdatePreferences {
  channel: UpdateChannel;
  automaticChecks: boolean;
  notifications: boolean;
  lastChecked: number;
  lastAttempt: number;
}
