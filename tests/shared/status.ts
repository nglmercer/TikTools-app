/** Cloud account states reported by `cloud.status`. */
export const ACCOUNT_STATE = {
  signedIn: 'signed-in',
  signedOut: 'signed-out',
  memoryOnly: 'memory-only',
} as const;
export type AccountState = (typeof ACCOUNT_STATE)[keyof typeof ACCOUNT_STATE];
