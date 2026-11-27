/** Machine-readable failure codes returned by the control API. */
export const ERROR_CODE = {
  revisionConflict: 'revision_conflict',
  notFound: 'not_found',
  tooLarge: 'too_large',
  capabilityUnavailable: 'capability_unavailable',
  automationNotFound: 'automation_not_found',
  commandPending: 'command_pending',
} as const;
export type ErrorCode = (typeof ERROR_CODE)[keyof typeof ERROR_CODE];
