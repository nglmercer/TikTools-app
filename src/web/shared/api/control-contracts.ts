import type { HostMessage } from '../../../shared/messages.ts';

export type RpcParams = Record<string, unknown>;

export interface RequestMetadata {
  expectedRevision?: string;
  commandId?: string;
  hostInstanceId?: string;
}
export interface ResourceVersion {
  kind: string;
  id: string;
  scope: string;
  revision: string;
}
export interface CommandReceipt {
  commandId: string;
  state: 'accepted' | 'running' | 'succeeded' | 'failed' | 'unknown';
  resource?: { kind: string; id: string; scope: string };
  committedAt?: number;
}
export interface ResponseMetadata {
  resourceVersions: ResourceVersion[];
  command?: CommandReceipt;
  hostInstanceId?: string;
  [field: string]: unknown;
}
export interface ControlCallOptions {
  signal?: AbortSignal;
  /** Optional bounded deadline for a call whose native contract is longer. */
  timeoutMs?: number;
  meta?: RequestMetadata;
  onMetadata?: (meta: ResponseMetadata | undefined) => void;
}
export interface CallResult<T> {
  result: T;
  meta?: ResponseMetadata;
}

/** Metadata-aware calls retain the domain payload's existing shape. */
export async function callWithMeta<T = unknown>(
  control: ControlClient, method: string, params?: RpcParams,
  options?: Omit<ControlCallOptions, 'onMetadata'>,
): Promise<CallResult<T>> {
  let meta: ResponseMetadata | undefined;
  const result = await control.call<T>(method, params, { ...options, onMetadata: (value) => { meta = value; } });
  return { result, meta };
}

export class ControlCallError extends Error {
  readonly code: string;
  readonly data?: unknown;

  constructor(code: string, message: string, data?: unknown) {
    super(message);
    this.name = 'ControlCallError';
    this.code = code;
    this.data = data;
  }
}

export function errorMessage(failure: unknown): string {
  return failure instanceof Error ? failure.message : String(failure);
}

export type TopicHandler<T = unknown> = (data: T) => void;
export type PushHandler = (message: HostMessage) => void;
export type TransportErrorHandler = (message: string) => void;
export type GapHandler = (lost: number) => void;
export type Unsubscribe = () => void;

export interface PresentationView {
  feed: boolean;
  behavior: boolean;
  points: boolean;
  visible: boolean;
}

export interface ControlClient {
  setPresentation?(view: PresentationView): Promise<void>;
  diagnostics?(): { pending: number; capacity: number; overloaded: number; topicCount?: number; topicSubscriptions?: number; pushSubscriptions?: number };
  attach(): void;
  detach(): void;
  call<T = unknown>(method: string, params?: RpcParams, options?: ControlCallOptions): Promise<T>;
  onTopic<T = unknown>(topic: string, handler: TopicHandler<T>): Unsubscribe;
  onPush(type: HostMessage['type'], handler: PushHandler): Unsubscribe;
  onTransportError(handler: TransportErrorHandler): Unsubscribe;
  onGap(handler: GapHandler): Unsubscribe;
}
