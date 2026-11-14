import type { PlaybackStatus } from './playback.ts';

export type ConnectionCase = 'normal' | 'offline' | 'auth';
export type FrameAction = { action: 'play' | 'step' | 'reset' } | { action: 'connection-case'; value: ConnectionCase };
export interface DemoFrameState {
  type: 'tiktools.demo.state'; instance: string; ready: boolean; paused: boolean; busy: boolean;
  events: number; status: PlaybackStatus; connectionCase: ConnectionCase;
}
const object = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value);
const validInstance = (value: unknown): value is string => typeof value === 'string' && /^[a-zA-Z0-9_-]{1,64}$/.test(value);
export const isConnectionCase = (value: unknown): value is ConnectionCase => value === 'normal' || value === 'offline' || value === 'auth';
export function isDemoFrameState(value: unknown): value is DemoFrameState {
  return object(value) && value.type === 'tiktools.demo.state' && validInstance(value.instance)
    && typeof value.ready === 'boolean' && typeof value.paused === 'boolean' && typeof value.busy === 'boolean'
    && typeof value.events === 'number' && Number.isInteger(value.events) && value.events >= 0 && value.events <= 300
    && typeof value.status === 'string' && ['ready', 'connecting', 'playing', 'paused', 'disconnected', 'limit', 'error'].includes(value.status)
    && isConnectionCase(value.connectionCase);
}
/** No general RPC or navigation is exposed to the parent document. */
export function parseFrameCommand(value: unknown, instance: string): FrameAction | null {
  if (!object(value) || value.type !== 'tiktools.demo.command' || value.instance !== instance || !validInstance(instance)) return null;
  if (value.action === 'play' || value.action === 'step' || value.action === 'reset') return { action: value.action };
  if (value.action === 'connection-case' && isConnectionCase(value.value)) return { action: value.action, value: value.value };
  return null;
}
