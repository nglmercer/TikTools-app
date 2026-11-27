/** Operations submitted to the automation library journal. */
export const LIBRARY_OPERATION = {
  applyTemplate: 'apply-template',
  createProfile: 'create-profile',
  assignRecord: 'assign-record',
  importTemplates: 'import-templates',
} as const;
export type LibraryOperation = (typeof LIBRARY_OPERATION)[keyof typeof LIBRARY_OPERATION];
