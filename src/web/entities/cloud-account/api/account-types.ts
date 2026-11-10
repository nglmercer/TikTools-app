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
  error?: string | null;
}

export interface CloudLoginUrlResult {
  browserUrl: string;
}
