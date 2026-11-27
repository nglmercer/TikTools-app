/** Automation node and action kinds that tests build by hand. */
export const NODE_TYPE = {
  pluginAction: 'plugin-action',
  refreshSource: 'refresh-source',
  actionStatus: 'action-status',
} as const;
export type NodeType = (typeof NODE_TYPE)[keyof typeof NODE_TYPE];
