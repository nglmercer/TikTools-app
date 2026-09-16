import type {
  JsonObject,
  JsonValue,
  NodeDefinition,
  PortDefinition,
  WorkflowEdge,
  WorkflowGraph,
  WorkflowNode,
} from '../../../automation/types.ts';

export function defaultNodeConfig(definition: NodeDefinition): JsonObject {
  switch (definition.type) {
    case 'trigger.event':
      return { eventType: 'tiktok.chat' };
    case 'condition.compare':
      return { leftPath: '', operator: 'equals', right: '' };
    case 'transform.template':
      return { template: '' };
    case 'transform.script':
      return { source: '' };
    case 'control.delay':
      return {};
    case 'control.cooldown':
      return {};
    case 'action.log':
      return { message: '' };
    case 'action.http':
      return {};
    case 'action.play-sound':
      return {};
    // Legacy quarantine: kept so older graphs keep their defaults. The host
    // catalog currently exposes no `action.tts` node; do not build on it.
    case 'action.tts':
      return { text: '' };
    case 'action.adjust-points':
      return {};
    default:
      return {};
  }
}

export function createWorkflowNode(definition: NodeDefinition, index: number): WorkflowNode {
  return {
    id: createId('node'),
    type: definition.type,
    version: definition.version,
    position: { x: 80, y: 80 + index * 190 },
    config: defaultNodeConfig(definition),
  };
}

export function createWorkflowGraph(
  name: string,
  eventType: string,
  triggerDefinition: NodeDefinition,
): WorkflowGraph {
  const trigger = createWorkflowNode(triggerDefinition, 0);
  trigger.id = createId('trigger');
  trigger.config = { eventType };
  return {
    schemaVersion: 1,
    id: createId('workflow'),
    name,
    enabled: false,
    nodes: [trigger],
    edges: [],
  };
}

export function appendNodeToGraph(
  graph: WorkflowGraph,
  node: WorkflowNode,
  definitions: NodeDefinition[],
): WorkflowGraph {
  const next = {
    ...graph,
    nodes: [...graph.nodes, { ...node, position: { ...node.position }, config: { ...node.config } }],
    edges: [...graph.edges],
  };
  const previous = graph.nodes[graph.nodes.length - 1];
  if (!previous) return next;
  const sourceDefinition = definitions.find((definition) => definition.type === previous.type);
  const targetDefinition = definitions.find((definition) => definition.type === node.type);
  const sourcePort = sourceDefinition ? preferredFlowOutput(sourceDefinition) : undefined;
  const targetPort = targetDefinition ? firstFlowInput(targetDefinition) : undefined;
  if (!sourcePort || !targetPort) return next;
  return {
    ...next,
    edges: [...next.edges, createFlowEdge(previous.id, sourcePort.name, node.id, targetPort.name)],
  };
}

export function removeNodeFromGraph(
  graph: WorkflowGraph,
  nodeId: string,
  definitions: NodeDefinition[],
): WorkflowGraph {
  const index = graph.nodes.findIndex((node) => node.id === nodeId);
  if (index < 0) return graph;
  const before = graph.nodes[index - 1];
  const after = graph.nodes[index + 1];
  const nodes = graph.nodes.filter((node) => node.id !== nodeId);
  let edges = graph.edges.filter((edge) => edge.source !== nodeId && edge.target !== nodeId);

  if (before && after) {
    const sourceDefinition = definitions.find((definition) => definition.type === before.type);
    const targetDefinition = definitions.find((definition) => definition.type === after.type);
    const sourcePort = sourceDefinition ? preferredFlowOutput(sourceDefinition) : undefined;
    const targetPort = targetDefinition ? firstFlowInput(targetDefinition) : undefined;
    if (sourcePort && targetPort && !edges.some((edge) => edge.kind === 'flow' && edge.source === before.id && edge.target === after.id)) {
      edges = [...edges, createFlowEdge(before.id, sourcePort.name, after.id, targetPort.name)];
    }
  }

  return { ...graph, nodes, edges };
}

export function normalizeWorkflowGraph(graph: WorkflowGraph, definitions: NodeDefinition[]): WorkflowGraph {
  const nodes = graph.nodes.map((node) => ({
    ...node,
    position: { ...node.position },
    config: { ...node.config },
  }));
  const nodeMap = new Map(nodes.map((node) => [node.id, node]));
  const definitionMap = new Map(definitions.map((definition) => [definition.type, definition]));
  const edges = graph.edges.filter((edge) => {
    const source = nodeMap.get(edge.source);
    const target = nodeMap.get(edge.target);
    const sourceDefinition = source ? definitionMap.get(source.type) : undefined;
    const targetDefinition = target ? definitionMap.get(target.type) : undefined;
    if (!source || !target || !sourceDefinition || !targetDefinition) return false;
    const sourcePort = findPort(sourceDefinition, edge.kind, edge.sourcePort, false);
    const targetPort = findPort(targetDefinition, edge.kind, edge.targetPort, true);
    if (!sourcePort || !targetPort) return false;
    return edge.kind === 'flow' || compatiblePorts(sourcePort, targetPort);
  }).map((edge) => ({ ...edge }));
  return { ...graph, nodes, edges };
}

export function firstFlowInput(definition: NodeDefinition): PortDefinition | undefined {
  return definition.inputs.find((port) => port.kind === 'flow');
}

export function preferredFlowOutput(definition: NodeDefinition): PortDefinition | undefined {
  const flowOutputs = definition.outputs.filter((port) => port.kind === 'flow');
  return flowOutputs.find((port) => port.name === 'flow')
    ?? flowOutputs.find((port) => port.name === 'success')
    ?? flowOutputs.find((port) => port.name === 'ready')
    ?? flowOutputs.find((port) => port.name === 'true')
    ?? flowOutputs[0];
}

export function createFlowEdge(source: string, sourcePort: string, target: string, targetPort: string): WorkflowEdge {
  return {
    id: createId('edge'),
    kind: 'flow',
    source,
    sourcePort,
    target,
    targetPort,
  };
}

function findPort(definition: NodeDefinition, kind: 'flow' | 'data', name: string, input: boolean): PortDefinition | undefined {
  return (input ? definition.inputs : definition.outputs).find((port) => port.kind === kind && port.name === name);
}

function compatiblePorts(source: PortDefinition, target: PortDefinition): boolean {
  if (!source.valueType || !target.valueType) return true;
  if (source.valueType === target.valueType) return true;
  return source.valueType === 'json' || target.valueType === 'json';
}

function createId(prefix: string): string {
  const randomUuid = globalThis.crypto?.randomUUID?.();
  return randomUuid ? `${prefix}-${randomUuid}` : `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

export function asString(value: JsonValue | undefined, fallback = ''): string {
  if (typeof value === 'string') return value;
  if (value === undefined || value === null) return fallback;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  if (!Array.isArray(value) && typeof value.path === 'string') return value.path;
  return JSON.stringify(value) ?? fallback;
}

export function asNumber(value: JsonValue | undefined, fallback = 0): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

export function asBoolean(value: JsonValue | undefined, fallback = false): boolean {
  return typeof value === 'boolean' ? value : fallback;
}
