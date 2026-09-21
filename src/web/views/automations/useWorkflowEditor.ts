import { ref, watch } from 'vue';

import type {
  AutomationEventType,
  JsonObject,
  NodeDefinition,
  WorkflowGraph,
} from '../../../automation/types.ts';
import type { AutomationWorkflowRecord } from '../../../shared/messages.ts';
import {
  appendNodeToGraph,
  createWorkflowGraph,
  createWorkflowNode,
  graphsEqual,
  prepareGraph,
  removeNodeFromGraph,
} from '../../components/node-editor/index.ts';

export type PendingWorkflowAction =
  | { kind: 'select'; record: AutomationWorkflowRecord }
  | { kind: 'create' }
  | { kind: 'template' };

export type WorkflowConfirmState =
  | { kind: 'discard'; action: PendingWorkflowAction }
  | { kind: 'delete'; id: string; name: string };

type WorkflowEditorSource = {
  readonly workflows: AutomationWorkflowRecord[];
  readonly nodes: NodeDefinition[];
  onSave: (graph: WorkflowGraph) => void;
  onDelete: (id: string) => void;
};

/** Draft selection, dirty tracking, node editing, and modal orchestration for the automations view. */
export function useWorkflowEditor(props: WorkflowEditorSource) {
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

  return {
    selectedId,
    draft,
    selectedNodeId,
    dirty,
    editorError,
    wizardOpen,
    templateOpen,
    pickerOpen,
    configuringNodeId,
    renameValue,
    confirmModal,
    applyWorkflowAction,
    selectWorkflow,
    requestCreateWorkflow,
    requestTemplateWorkflow,
    handleCreateWorkflow,
    handleCreateFromTemplate,
    handleRename,
    handleAddNode,
    handleDeleteNode,
    handleConfigChange,
    openConfiguration,
    handleSave,
    handleDeleteWorkflow,
    confirmDelete,
  };
}

export type WorkflowEditor = ReturnType<typeof useWorkflowEditor>;
