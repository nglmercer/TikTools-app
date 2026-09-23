import type {
  CreatorRecord,
  PointsConfig,
  TopViewerPayload,
  UiEvent,
  ViewerRecord,
} from '../shared/messages.ts';
import type { JsonObject } from '../automation/types.ts';

export type BuiltinAppTab = 'feed' | 'points' | 'analytics' | 'connect' | 'behavior' | 'plugins' | 'widgets' | 'settings';

/** Builtin tabs plus plugin page tabs (`plugin:<pluginId>:<pageId>`). */
export type AppTab = BuiltinAppTab | `plugin:${string}:${string}`;

export type ConnectionStatus =
  | 'idle'
  | 'connecting'
  | 'connected'
  | 'retrying'
  | 'disconnected'
  | 'error';

export type DisplayEvent = UiEvent & {
  id: number;
  receivedAt: number;
};

export type EventFilter = 'all' | 'chat' | 'gift' | 'like' | 'social';

export type PluginSettingsState = {
  schema: JsonObject;
  uiHints?: JsonObject;
  values: JsonObject;
};

export type { CreatorRecord, PointsConfig, TopViewerPayload, ViewerRecord };
