declare module "tiktools:events" {
  export interface PushEvent {
    type: string;
    data?: unknown;
  }
  export function emit(event: PushEvent): void;
  export function emitMany(events: PushEvent[]): number;
}
