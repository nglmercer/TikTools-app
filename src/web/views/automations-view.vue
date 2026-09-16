<script lang="tsx">
import { ref, watch } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type {
  AutomationEventType,
  AutomationEvent,
  AutomationScriptAnalysis,
  JsonObject,
  NodeDefinition,
  WorkflowGraph,
} from '../../automation/types.ts';
import type { AutomationWorkflowRecord } from '../../shared/messages.ts';
import { IconEdit, IconPlus, IconSparkles, IconTemplate, IconTrash } from '../components/icons.vue';
import { Alert, Badge, EmptyState } from '../components/ui/Card.vue';
import { Button } from '../components/ui/Button.vue';
import { Checkbox } from '../components/ui/Checkbox.vue';
import { ConfirmModal, TextPromptModal } from '../components/ui/Modal.vue';
import { PageHeader } from '../components/ui/Page.vue';
import {
  NodeConfigModal,
  NodePickerModal,
  WorkflowCanvas,
  WorkflowTemplateModal,
  WorkflowWizardModal,
  appendNodeToGraph,
  createWorkflowGraph,
  createWorkflowNode,
  normalizeWorkflowGraph,
  removeNodeFromGraph,
} from '../components/node-editor/index.ts';
import { t, type Locale } from '../i18n.ts';
import type { OpenMediaPicker } from '../../shared/messages.ts';

type AutomationsViewProps = {
  locale: Locale;
  workflows: AutomationWorkflowRecord[];
  nodes: NodeDefinition[];
  error?: string;
  scriptAnalysis?: AutomationScriptAnalysis;
  lastEvent?: AutomationEvent;
  lastEventCapturedAt?: number;
  onRefresh: () => void;
  onSave: (graph: WorkflowGraph) => void;
  onDelete: (id: string) => void;
  onSetEnabled: (id: string, enabled: boolean) => void;
  onAnalyzeScript: (nodeId: string, source: string, offset: number, eventType?: AutomationEventType) => void;
  onOpenMediaPicker?: OpenMediaPicker;
};

type PendingWorkflowAction =
  | { kind: 'select'; record: AutomationWorkflowRecord }
  | { kind: 'create' }
  | { kind: 'template' };

type WorkflowConfirmState =
  | { kind: 'discard'; action: PendingWorkflowAction }
  | { kind: 'delete'; id: string; name: string };

export const AutomationsView = defineVueComponent<AutomationsViewProps>(
  ['locale', 'workflows', 'nodes', 'error', 'scriptAnalysis', 'lastEvent', 'lastEventCapturedAt', 'onRefresh', 'onSave', 'onDelete', 'onSetEnabled', 'onAnalyzeScript', 'onOpenMediaPicker'],
  (props) => {
  const initialRecord = props.workflows[0];
  const initialGraph = initialRecord ? prepareGraph(initialRecord.graph, props.nodes) : null;
  const selectedId = ref<string | null>(initialRecord?.id ?? null);
  const draft = ref<WorkflowGraph | null>(initialGraph);
  const selectedNodeId = ref<string | null>(initialGraph?.nodes[0]?.id ?? null);
  const dirty = ref(false);
  const editorError = ref('');
  const wizardOpen = ref(false);
  const templateOpen = ref(false);
  const pickerOpen = ref(false);
  const configuringNodeId = ref<string | null>(null);
  const renameValue = ref<string | null>(null);
  const confirmModal = ref<WorkflowConfirmState | null>(null);

  watch(() => [selectedId.value, props.workflows, props.nodes, dirty.value], () => {
    if (selectedId.value || props.workflows.length === 0 || dirty.value) return;
    const first = props.workflows[0];
    if (!first) return;
    const nextGraph = prepareGraph(first.graph, props.nodes);
    selectedId.value = first.id;
    draft.value = nextGraph;
    selectedNodeId.value = nextGraph.nodes[0]?.id ?? null;
  });

  watch(() => [selectedId.value, props.workflows, props.nodes, dirty.value], () => {
    if (!selectedId.value || dirty.value) return;
    const record = props.workflows.find((workflow) => workflow.id === selectedId.value);
    if (!record) return;
    const nextGraph = prepareGraph(record.graph, props.nodes);
    draft.value = nextGraph;
    selectedNodeId.value = nextGraph.nodes.some((node) => node.id === selectedNodeId.value) ? selectedNodeId.value : nextGraph.nodes[0]?.id ?? null;
  });

  watch(() => [draft.value, dirty.value, selectedId.value, props.workflows], () => {
    if (!dirty.value || !draft.value || !selectedId.value) return;
    const record = props.workflows.find((workflow) => workflow.id === selectedId.value);
    if (record && graphsEqual(record.graph, draft.value)) dirty.value = false;
  });

  watch(() => [draft.value?.id, draft.value?.nodes.length, selectedNodeId.value], () => {
    if (!draft.value?.nodes.length) {
      selectedNodeId.value = null;
      return;
    }
    if (!selectedNodeId.value || !draft.value.nodes.some((node) => node.id === selectedNodeId.value)) {
      selectedNodeId.value = draft.value.nodes[0]?.id ?? null;
    }
  });

  const updateDraft = (update: (current: WorkflowGraph) => WorkflowGraph): void => {
    if (draft.value) draft.value = update(draft.value);
    dirty.value = true;
    editorError.value = '';
  };

  const applyWorkflowAction = (action: PendingWorkflowAction): void => {
    if (action.kind === 'create') {
      wizardOpen.value = true;
      return;
    }
    if (action.kind === 'template') {
      templateOpen.value = true;
      return;
    }
    const nextGraph = prepareGraph(action.record.graph, props.nodes);
    selectedId.value = action.record.id;
    draft.value = nextGraph;
    selectedNodeId.value = nextGraph.nodes[0]?.id ?? null;
    configuringNodeId.value = null;
    dirty.value = !graphsEqual(action.record.graph, nextGraph);
    editorError.value = '';
  };

  const requestWorkflowAction = (action: PendingWorkflowAction): void => {
    if (dirty.value) {
      confirmModal.value = { kind: 'discard', action };
      return;
    }
    applyWorkflowAction(action);
  };

  const selectWorkflow = (record: AutomationWorkflowRecord): void => {
    if (record.id === selectedId.value) return;
    requestWorkflowAction({ kind: 'select', record });
  };

  const requestCreateWorkflow = (): void => requestWorkflowAction({ kind: 'create' });

  const requestTemplateWorkflow = (): void => requestWorkflowAction({ kind: 'template' });

  const handleCreateWorkflow = (name: string, eventType: AutomationEventType): void => {
    const triggerDefinition = props.nodes.find((definition) => definition.type === 'trigger.event');
    if (!triggerDefinition) {
      editorError.value = 'The Event Trigger node is not available. Refresh the node catalog.';
      return;
    }
    const graph = createWorkflowGraph(name, eventType, triggerDefinition);
    wizardOpen.value = false;
    selectedId.value = graph.id;
    draft.value = graph;
    selectedNodeId.value = graph.nodes[0]?.id ?? null;
    configuringNodeId.value = null;
    dirty.value = true;
    editorError.value = '';
  };

  const handleCreateFromTemplate = (graph: WorkflowGraph): void => {
    templateOpen.value = false;
    selectedId.value = graph.id;
    draft.value = graph;
    selectedNodeId.value = graph.nodes[1]?.id ?? graph.nodes[0]?.id ?? null;
    configuringNodeId.value = null;
    dirty.value = true;
    editorError.value = '';
  };

  const handleRename = (name: string): void => {
    renameValue.value = null;
    updateDraft((current) => ({ ...current, name }));
  };

  const handleAddNode = (definition: NodeDefinition): void => {
    if (!draft.value || definition.kind === 'trigger') return;
    const node = createWorkflowNode(definition, draft.value.nodes.length);
    const nextGraph = appendNodeToGraph(draft.value, node, props.nodes);
    draft.value = nextGraph;
    selectedNodeId.value = node.id;
    configuringNodeId.value = null;
    pickerOpen.value = false;
    dirty.value = true;
    editorError.value = '';
  };

  const handleDeleteNode = (nodeId: string): void => {
    if (!draft.value) return;
    const node = draft.value.nodes.find((item) => item.id === nodeId);
    if (!node || node.type === 'trigger.event') return;
    const nodeIndex = draft.value.nodes.findIndex((item) => item.id === nodeId);
    const nextGraph = removeNodeFromGraph(draft.value, nodeId, props.nodes);
    draft.value = nextGraph;
    selectedNodeId.value = nextGraph.nodes[Math.max(0, nodeIndex - 1)]?.id ?? nextGraph.nodes[0]?.id ?? null;
    if (configuringNodeId.value === nodeId) configuringNodeId.value = null;
    dirty.value = true;
  };

  const handleConfigChange = (config: JsonObject): void => {
    const nodeId = selectedNodeId.value;
    if (!nodeId) return;
    updateDraft((current) => ({
      ...current,
      nodes: current.nodes.map((node) => node.id === nodeId ? { ...node, config: { ...config } } : node),
    }));
  };

  const openConfiguration = (): void => {
    if (selectedNodeId.value) configuringNodeId.value = selectedNodeId.value;
  };

  const handleSave = (): void => {
    if (!draft.value) return;
    props.onSave(prepareGraph(draft.value, props.nodes));
  };

  const handleDeleteWorkflow = (): void => {
    const record = selectedId.value ? props.workflows.find((workflow) => workflow.id === selectedId.value) : undefined;
    if (!selectedId.value || !record) return;
    confirmModal.value = { kind: 'delete', id: selectedId.value, name: record.name };
  };

  const confirmDelete = (id: string): void => {
    confirmModal.value = null;
    props.onDelete(id);
    selectedId.value = null;
    draft.value = null;
    selectedNodeId.value = null;
    configuringNodeId.value = null;
    dirty.value = false;
  };

  return () => {
  const locale = props.locale;
  const workflows = props.workflows;
  const nodes = props.nodes;
  const error = props.error;
  const scriptAnalysis = props.scriptAnalysis;
  const lastEvent = props.lastEvent;
  const lastEventCapturedAt = props.lastEventCapturedAt;
  const draftValue = draft.value;
  const selectedRecord = selectedId.value ? workflows.find((workflow) => workflow.id === selectedId.value) : undefined;
  const selectedNode = draftValue?.nodes.find((node) => node.id === selectedNodeId.value);
  const selectedDefinition = selectedNode ? nodes.find((node) => node.type === selectedNode.type) : undefined;
  const configuringNode = draftValue?.nodes.find((node) => node.id === configuringNodeId.value);
  const configuringDefinition = configuringNode ? nodes.find((node) => node.type === configuringNode.type) : undefined;
  const dirtyValue = dirty.value;
  const confirmValue = confirmModal.value;
  const rename = renameValue.value;
  const wizard = wizardOpen.value;
  const template = templateOpen.value;
  const picker = pickerOpen.value;

  return (
    <main class="automation-view">
      <PageHeader
        title={t(locale, 'automations')}
        subtitle={t(locale, 'automationsLead')}
        icon={<IconSparkles />}
        action={
          <div class="automation-header-actions">
            <Button variant="ghost" size="sm" onClick={props.onRefresh}>{t(locale, 'refresh')}</Button>
            <Button variant="soft" size="sm" icon={<IconTemplate size={14} />} onClick={requestTemplateWorkflow}>{t(locale, 'templates')}</Button>
            <Button variant="primary" size="sm" onClick={requestCreateWorkflow}>{t(locale, 'newWorkflow')}</Button>
          </div>
        }
      />

      {error ? <Alert variant="danger">{error}</Alert> : null}
      {editorError.value ? <Alert variant="warning">{editorError.value}</Alert> : null}

      <div class="automation-workspace automation-workspace--simple">
        <aside class="automation-sidebar">
          <div class="automation-panel-heading">
            <span>{t(locale, 'automations')}</span>
            <Badge tone="cyan">{workflows.length}</Badge>
          </div>

          {workflows.length === 0 ? (
            <EmptyState title={t(locale, 'noWorkflows')} action={<Button variant="soft" size="sm" onClick={requestCreateWorkflow}>{t(locale, 'newWorkflow')}</Button>} />
          ) : (
            <div class="automation-workflow-list">
              {workflows.map((record) => (
                <button key={record.id} type="button" class={`automation-workflow-item ${selectedId.value === record.id ? 'is-active' : ''}`} onClick={() => selectWorkflow(record)}>
                  <span class="automation-workflow-item__name">{record.name}</span>
                  <span class="automation-workflow-item__meta">
                    <span>{t(locale, 'nodeCount', { count: record.graph.nodes.length })}</span>
                    <span class={`automation-status-dot ${record.enabled ? 'is-enabled' : ''}`} />
                  </span>
                </button>
              ))}
            </div>
          )}

          <div class="automation-catalog">
            <div class="automation-panel-heading">
              <span>{t(locale, 'nodeCatalog')}</span>
              <Badge>{Math.max(0, nodes.filter((node) => node.kind !== 'trigger').length)}</Badge>
            </div>
            <p class="automation-panel-hint">{t(locale, 'automationAddNodeHint')}</p>
            <Button variant="cyan" block disabled={!draftValue} icon={<IconPlus size={14} />} onClick={() => { pickerOpen.value = true; }}>{t(locale, 'addStep')}</Button>
            <p class="automation-panel-hint automation-panel-hint--secondary">{t(locale, 'automationFlowHint')}</p>
          </div>
        </aside>

        <section class="automation-editor-panel">
          {!draftValue ? (
            <EmptyState title={t(locale, 'selectWorkflow')} description={t(locale, 'noWorkflows')} action={<Button variant="primary" onClick={requestCreateWorkflow}>{t(locale, 'newWorkflow')}</Button>} />
          ) : (
            <>
              <div class="automation-editor-toolbar">
                <div class="automation-workflow-name">
                  <button type="button" class="automation-workflow-name-button" onClick={() => { renameValue.value = draftValue.name; }}>
                    <span class="automation-workflow-name-button__value">{draftValue.name}</span>
                    <span class="automation-workflow-name-button__edit" aria-hidden="true"><IconEdit size={12} /></span>
                  </button>
                  {dirtyValue ? <Badge tone="pink">{t(locale, 'unsavedChanges')}</Badge> : null}
                </div>
                <div class="automation-toolbar-actions">
                  {selectedRecord ? (
                    <Checkbox checked={draftValue.enabled} onCheckedChange={(enabled) => { draft.value = { ...draft.value!, enabled }; props.onSetEnabled(selectedRecord.id, enabled); }} label={draftValue.enabled ? t(locale, 'disableWorkflow') : t(locale, 'enableWorkflow')} />
                  ) : null}
                  <Button variant="primary" size="sm" disabled={!dirtyValue} onClick={handleSave}>{t(locale, 'saveWorkflow')}</Button>
                  {selectedRecord ? <Button variant="danger" size="sm" icon={<IconTrash />} iconOnly tooltip={t(locale, 'deleteWorkflow')} onClick={handleDeleteWorkflow} /> : null}
                </div>
              </div>

              <WorkflowCanvas
                locale={locale}
                graph={draftValue}
                definitions={nodes}
                selectedNodeId={selectedNodeId.value}
                onSelectNode={(id) => { selectedNodeId.value = id; }}
                onAddNode={() => { pickerOpen.value = true; }}
                onDeleteNode={handleDeleteNode}
              />
            </>
          )}
        </section>

        <aside class="automation-inspector">
          <div class="automation-panel-heading">{t(locale, 'nodeInspector')}</div>
          {!selectedNode ? (
            <p class="automation-panel-hint">{t(locale, 'noNodeSelected')}</p>
          ) : (
            <div class="automation-inspector-content">
              <div class="automation-node-title">{selectedDefinition?.title ?? selectedNode.type}</div>
              <div class="automation-node-type">
                <span>{selectedDefinition?.category ?? 'Plugin'}</span>
                <span>·</span>
                <span>{selectedDefinition?.kind ?? 'node'}</span>
              </div>
              <div class="automation-inspector-summary">
                <span class="automation-inspector-summary__label">{t(locale, 'workflowSteps')}</span>
                <strong>{selectedNode.type}</strong>
              </div>
              <Button variant="primary" block onClick={openConfiguration}>{t(locale, 'configureStep')}</Button>
              <p class="automation-panel-hint">{t(locale, 'configureStepHint')}</p>
              {selectedDefinition?.requiredCapabilities?.length ? (
                <div class="automation-capabilities">
                  <span>{t(locale, 'capabilities')}</span>
                  {selectedDefinition.requiredCapabilities.map((capability) => <Badge key={capability}>{capability}</Badge>)}
                </div>
              ) : null}
            </div>
          )}
        </aside>
      </div>

      {wizard ? <WorkflowWizardModal locale={locale} onClose={() => { wizardOpen.value = false; }} onCreate={handleCreateWorkflow} /> : null}
      {template ? <WorkflowTemplateModal locale={locale} definitions={nodes} onClose={() => { templateOpen.value = false; }} onCreate={handleCreateFromTemplate} onOpenMediaPicker={props.onOpenMediaPicker} /> : null}
      {picker ? <NodePickerModal locale={locale} definitions={nodes} onClose={() => { pickerOpen.value = false; }} onSelect={handleAddNode} /> : null}
      {configuringNode ? (
        <NodeConfigModal
          locale={locale}
          node={configuringNode}
          definition={configuringDefinition}
          eventType={eventTypeForGraph(draftValue)}
          lastEvent={lastEvent}
          lastEventCapturedAt={lastEventCapturedAt}
          analysis={scriptAnalysis?.nodeId === configuringNode.id ? scriptAnalysis : undefined}
          onApply={(config) => {
            handleConfigChange(config);
            configuringNodeId.value = null;
          }}
          onAnalyzeScript={props.onAnalyzeScript}
          onOpenMediaPicker={props.onOpenMediaPicker}
          onClose={() => { configuringNodeId.value = null; }}
        />
      ) : null}
      {rename !== null ? (
        <TextPromptModal
          title={t(locale, 'renameWorkflowTitle')}
          description={t(locale, 'workflowNameHint')}
          label={t(locale, 'workflowName')}
          initialValue={rename}
          placeholder={t(locale, 'workflowNamePlaceholder')}
          confirmLabel={t(locale, 'confirm')}
          cancelLabel={t(locale, 'cancel')}
          requiredMessage={t(locale, 'workflowNameRequired')}
          onConfirm={handleRename}
          onClose={() => { renameValue.value = null; }}
        />
      ) : null}
      {confirmValue?.kind === 'discard' ? (
        <ConfirmModal
          title={t(locale, 'discardWorkflowTitle')}
          description={t(locale, 'discardWorkflowChanges')}
          confirmLabel={t(locale, 'discardChanges')}
          cancelLabel={t(locale, 'cancel')}
          onConfirm={() => {
            const pending = confirmValue;
            confirmModal.value = null;
            dirty.value = false;
            applyWorkflowAction(pending.action);
          }}
          onClose={() => { confirmModal.value = null; }}
        />
      ) : null}
      {confirmValue?.kind === 'delete' ? (
        <ConfirmModal
          title={t(locale, 'deleteWorkflow')}
          description={t(locale, 'deleteWorkflowConfirm', { name: confirmValue.name })}
          confirmLabel={t(locale, 'deleteWorkflow')}
          cancelLabel={t(locale, 'cancel')}
          danger
          onConfirm={() => confirmDelete(confirmValue.id)}
          onClose={() => { confirmModal.value = null; }}
        />
      ) : null}
    </main>
  );
  };
  },
);

function prepareGraph(graph: WorkflowGraph, definitions: NodeDefinition[]): WorkflowGraph {
  const cloned = cloneGraph(graph);
  return definitions.length > 0 ? normalizeWorkflowGraph(cloned, definitions) : cloned;
}

function cloneGraph(graph: WorkflowGraph): WorkflowGraph {
  return {
    ...graph,
    nodes: graph.nodes.map((node) => ({ ...node, position: { ...node.position }, config: { ...node.config } })),
    edges: graph.edges.map((edge) => ({ ...edge })),
  };
}

function graphsEqual(left: WorkflowGraph, right: WorkflowGraph): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function eventTypeForGraph(graph: WorkflowGraph | null): AutomationEventType | undefined {
  const trigger = graph?.nodes.find((node) => node.type === 'trigger.event');
  const eventType = trigger?.config.eventType;
  if (typeof eventType !== 'string') return undefined;
  return eventType as AutomationEventType;
}

export default AutomationsView;
</script>
