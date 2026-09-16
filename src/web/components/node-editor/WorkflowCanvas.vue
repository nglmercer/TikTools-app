<script lang="tsx">
import type { NodeDefinition, WorkflowEdge, WorkflowGraph, WorkflowNode } from '../../../automation/types.ts';
import { IconChevronRight, IconPlus, IconTrash } from '../icons.vue';
import { Icon } from '../icons/Icon.vue';
import { presentationForEvent } from '../icons/event-icons.ts';
import { Badge, EmptyState } from '../ui/Card.vue';
import { Button } from '../ui/Button.vue';
import { t, type Locale } from '../../i18n.ts';
import { asString } from './graph.ts';

type WorkflowCanvasProps = {
  locale: Locale;
  graph: WorkflowGraph;
  definitions: NodeDefinition[];
  selectedNodeId: string | null;
  onSelectNode: (nodeId: string) => void;
  onAddNode: () => void;
  onDeleteNode: (nodeId: string) => void;
};

export function WorkflowCanvas({
  locale,
  graph,
  definitions,
  selectedNodeId,
  onSelectNode,
  onAddNode,
  onDeleteNode,
}: WorkflowCanvasProps) {
  const definitionMap = new Map(definitions.map((definition) => [definition.type, definition]));
  const orderedNodes = orderNodes(graph, definitionMap);

  if (orderedNodes.length === 0) {
    return (
      <div class="node-editor-canvas node-editor-canvas--empty">
        <EmptyState title={t(locale, 'emptyWorkflowTitle')} description={t(locale, 'emptyWorkflowHint')} action={<Button variant="primary" onClick={onAddNode}>{t(locale, 'addStep')}</Button>} />
      </div>
    );
  }

  return (
    <div class="node-editor-canvas">
      <div class="node-editor-canvas__intro">
        <div>
          <strong>{t(locale, 'workflowSteps')}</strong>
          <span>{t(locale, 'workflowStepsHint')}</span>
        </div>
        <Badge tone="cyan">{t(locale, 'nodeCount', { count: orderedNodes.length })}</Badge>
      </div>

      <div class="node-editor-flow-list">
        {orderedNodes.map((node, index) => {
          const definition = definitionMap.get(node.type);
          const next = orderedNodes[index + 1];
          const edge = next ? graph.edges.find((candidate) => candidate.kind === 'flow' && candidate.source === node.id && candidate.target === next.id) : undefined;
          return (
            <div key={node.id} class="node-editor-flow-item">
              <NodeCard
                locale={locale}
                node={node}
                definition={definition}
                index={index}
                selected={selectedNodeId === node.id}
                canDelete={definition?.kind !== 'trigger'}
                onSelect={() => onSelectNode(node.id)}
                onDelete={() => onDeleteNode(node.id)}
              />
              {next ? <FlowConnector locale={locale} edge={edge} /> : null}
            </div>
          );
        })}
      </div>

      <button type="button" class="node-editor-add-step" onClick={onAddNode}>
        <span class="node-editor-add-step__icon"><IconPlus size={14} /></span>
        <span>{t(locale, 'addStep')}</span>
      </button>
    </div>
  );
}

function NodeCard({
  locale,
  node,
  definition,
  index,
  selected,
  canDelete,
  onSelect,
  onDelete,
}: {
  locale: Locale;
  node: WorkflowNode;
  definition?: NodeDefinition;
  index: number;
  selected: boolean;
  canDelete: boolean;
  onSelect: () => void;
  onDelete: () => void;
}) {
  const kind = definition?.kind ?? 'plugin';
  const triggerIcon = node.type === 'trigger.event'
    ? presentationForEvent(asString(node.config.eventType, '*')).icon
    : undefined;
  return (
    <article class={`node-editor-node-card ${selected ? 'is-selected' : ''} is-${kind}`}>
      <div class="node-editor-node-card__main">
        <button
          type="button"
          class="node-editor-node-card__select"
          onClick={onSelect}
          aria-pressed={selected}
        >
          <span class="node-editor-node-card__number">{index + 1}</span>
          <span class="node-editor-node-card__content">
            <span class="node-editor-node-card__topline">
              {triggerIcon ? <Icon name={triggerIcon} size={14} /> : null}
              <strong>{definition?.title ?? node.type}</strong>
              <Badge tone={kind === 'action' ? 'pink' : kind === 'trigger' ? 'cyan' : 'neutral'}>{kind}</Badge>
            </span>
            <span class="node-editor-node-card__type">{node.type}</span>
            <span class="node-editor-node-card__summary">{nodeSummary(node, definition, locale)}</span>
          </span>
          <span class="node-editor-node-card__chevron"><IconChevronRight size={16} /></span>
        </button>
        <div class="node-editor-node-card__actions">
          {canDelete ? <Button variant="ghost" size="sm" icon={<IconTrash />} iconOnly tooltip={t(locale, 'removeStep')} onClick={onDelete} /> : <span class="node-editor-node-card__trigger-label">{t(locale, 'triggerStep')}</span>}
        </div>
      </div>
    </article>
  );
}

function FlowConnector({ locale, edge }: { locale: Locale; edge?: WorkflowEdge }) {
  return (
    <div class={`node-editor-flow-connector ${edge ? '' : 'is-disconnected'}`} aria-hidden="true">
      <span />
      <small>{edge ? t(locale, 'nextStep') : t(locale, 'notConnected')}</small>
      <span />
    </div>
  );
}

function orderNodes(graph: WorkflowGraph, definitions: Map<string, NodeDefinition>): WorkflowNode[] {
  const byId = new Map(graph.nodes.map((node) => [node.id, node]));
  const next = new Map<string, string>();
  for (const edge of graph.edges) {
    if (edge.kind === 'flow' && !next.has(edge.source) && byId.has(edge.target)) next.set(edge.source, edge.target);
  }

  const trigger = graph.nodes.find((node) => definitions.get(node.type)?.kind === 'trigger');
  const ordered: WorkflowNode[] = [];
  const visited = new Set<string>();
  let current = trigger;
  while (current && !visited.has(current.id)) {
    ordered.push(current);
    visited.add(current.id);
    current = byId.get(next.get(current.id) ?? '');
  }
  for (const node of graph.nodes) {
    if (!visited.has(node.id)) ordered.push(node);
  }
  return ordered;
}

function nodeSummary(node: WorkflowNode, definition: NodeDefinition | undefined, locale: Locale): string {
  const config = node.config;
  switch (node.type) {
    case 'trigger.event': return eventLabel(asString(config.eventType, '*'), locale);
    case 'condition.compare': return `${asString(config.leftPath, 'event.data')}  ${operatorLabel(asString(config.operator, 'equals'))}  ${asString(config.right)}`;
    case 'transform.template': return asString(config.template);
    case 'transform.script': return t(locale, 'nodeTransformScriptSummary');
    case 'control.delay': return `${asString(config.delayMs, '0')} ms`;
    case 'control.cooldown': return `${asString(config.durationMs, '0')} ms · ${asString(config.key)}`;
    case 'action.log': return asString(config.message);
    case 'action.http': return `${asString(config.method, 'GET')} ${asString(config.url)}`;
    case 'action.play-sound': return asString(config.filePath);
    // Legacy quarantine: see NodeConfigForm. Unreachable from the catalog.
    case 'action.tts': return asString(config.text);
    case 'action.adjust-points': return `${asString(config.uniqueId)} · ${asString(config.delta, '0')}`;
    default: return definition?.title ?? node.type;
  }
}

function operatorLabel(value: string): string {
  const labels: Record<string, string> = {
    equals: '=',
    'not-equals': '≠',
    'greater-than': '>',
    'greater-or-equal': '≥',
    'less-than': '<',
    'less-or-equal': '≤',
    contains: 'contains',
    'starts-with': 'starts with',
    truthy: 'is true',
    falsy: 'is false',
  };
  return labels[value] ?? value;
}

function eventLabel(value: string, locale: Locale): string {
  const key = `workflow.event.${value}`;
  const translated = t(locale, key);
  return translated !== key ? translated : value;
}

export default WorkflowCanvas;
</script>
