export interface SyncRecoveryContext {
  generation(): number;
  available(): boolean;
}
