/** JSON-safe values and automation contracts are generated from Rust. */
import type {
  AutomationCreator as GeneratedAutomationCreator,
  AutomationEvent as GeneratedAutomationEvent,
  AutomationPoints as GeneratedAutomationPoints,
  AutomationUser as GeneratedAutomationUser,
  ConnectionAutomationData,
  PointsAwardedAutomationData,
} from './contracts/generated/automation-events.ts';
import type {
  JsonObject,
  JsonValue,
} from './contracts/generated/json-value.ts';
import type { AutomationEventType } from './contracts/events.ts';

export type { JsonArray, JsonObject, JsonPrimitive, JsonValue } from './contracts/generated/json-value.ts';
export type { AutomationEventType } from './contracts/events.ts';
export type AutomationUser = GeneratedAutomationUser & JsonObject;
export type AutomationCreator = GeneratedAutomationCreator & JsonObject;
export type AutomationPoints = GeneratedAutomationPoints & JsonObject;
export type ConnectionData = ConnectionAutomationData & JsonObject;
export type PointsAwardedData = PointsAwardedAutomationData & JsonObject;

export type AutomationEvent<T extends JsonValue = JsonValue> =
  Omit<GeneratedAutomationEvent, 'type' | 'data'> & JsonObject & {
    type: AutomationEventType;
    data: T;
  };

export type PortKind = 'flow' | 'data';

export type ValueType =
  | 'string'
  | 'number'
  | 'boolean'
  | 'json'
  | 'event'
  | 'bytes'
  | 'audio-ref'
  | 'secret-ref';

export interface PortDefinition {
  name: string;
  title: string;
  kind: PortKind;
  valueType?: ValueType;
  required?: boolean;
  multiple?: boolean;
}

export interface NodeDefinition {
  type: string;
  version: number;
  pluginId: string;
  title: string;
  category: string;
  kind: 'trigger' | 'condition' | 'transform' | 'action' | 'control';
  inputs: PortDefinition[];
  outputs: PortDefinition[];
  configSchema: JsonObject;
  requiredCapabilities?: string[];
  /** Optional fast-path trigger filter for worker-backed trigger nodes. */
  triggerTypes?: AutomationEventType[];
}

export interface WorkflowNode {
  id: string;
  type: string;
  version: number;
  position: { x: number; y: number };
  config: JsonObject;
  disabled?: boolean;
}

export interface WorkflowEdge {
  id: string;
  kind: PortKind;
  source: string;
  sourcePort: string;
  target: string;
  targetPort: string;
}

export interface WorkflowGraph {
  schemaVersion: 1;
  id: string;
  name: string;
  enabled: boolean;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
}

export interface AutomationScriptDiagnostic {
  line: number;
  column: number;
  message: string;
  severity: string;
}

export interface AutomationScriptCompletion {
  label: string;
  kind: string;
  detail?: string;
  documentation?: string;
  path?: string;
  value?: JsonValue;
  valueSource?: 'live-event' | 'sample-event';
}

export interface AutomationScriptHover {
  detail: string;
  documentation?: string;
  path?: string;
  value?: JsonValue;
  valueSource?: 'live-event' | 'sample-event';
}

export interface AutomationScriptAnalysis {
  nodeId: string;
  source: string;
  diagnostics: AutomationScriptDiagnostic[];
  completions: AutomationScriptCompletion[];
  hover?: AutomationScriptHover;
}
