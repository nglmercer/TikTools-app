/** Wire message types exchanged with the native host. */
export const MESSAGE_TYPE = {
  rpcResponse: 'rpc-response',
} as const;
export type MessageType = (typeof MESSAGE_TYPE)[keyof typeof MESSAGE_TYPE];
