<script lang="tsx">
import { defineVueComponent } from '../vue/component.ts';

import type {
  AutomationEventType,
  AutomationEvent,
  AutomationScriptAnalysis,
  NodeDefinition,
  WorkflowGraph,
} from '../../automation/types.ts';
import type { AutomationWorkflowRecord } from '../../shared/messages.ts';
import { IconEdit, IconSparkles, IconTemplate, IconTrash } from '../components/icons.vue';
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
  eventTypeForGraph,
} from '../components/node-editor/index.ts';
import { t, type Locale } from '../i18n.ts';
import type { OpenMediaPicker } from '../../shared/messages.ts';
import { useWorkflowEditor } from './automations/useWorkflowEditor.ts';
import { WorkflowSidebar } from './automations/WorkflowSidebar.vue';
import { NodeInspector } from './automations/NodeInspector.vue';

type AutomationsViewProps = {
  locale: Locale;
  workflows: AutomationWorkflowRecord[];
  nodes: NodeDefinition[];
  /** Host-stamped plugin templates forwarded to the template modal. */
  pluginTemplates?: unknown[];
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

export const AutomationsView = defineVueComponent<AutomationsViewProps>(
  ['locale', 'workflows', 'nodes', 'pluginTemplates', 'error', 'scriptAnalysis', 'lastEvent', 'lastEventCapturedAt', 'onRefresh', 'onSave', 'onDelete', 'onSetEnabled', 'onAnalyzeScript', 'onOpenMediaPicker'],
  (props) => {
  const editor = useWorkflowEditor(props);

  return () => {
  const locale = props.locale;
  const workflows = props.workflows;
  const nodes = props.nodes;
  const error = props.error;
  const scriptAnalysis = props.scriptAnalysis;
  const lastEvent = props.lastEvent;
  const lastEventCapturedAt = props.lastEventCapturedAt;
  const draftValue = editor.draft.value;
  const selectedRecord = editor.selectedId.value ? workflows.find((workflow) => workflow.id === editor.selectedId.value) : undefined;
  const selectedNode = draftValue?.nodes.find((node) => node.id === editor.selectedNodeId.value);
  const selectedDefinition = selectedNode ? nodes.find((node) => node.type === selectedNode.type) : undefined;
  const configuringNode = draftValue?.nodes.find((node) => node.id === editor.configuringNodeId.value);
  const configuringDefinition = configuringNode ? nodes.find((node) => node.type === configuringNode.type) : undefined;
  const dirtyValue = editor.dirty.value;
  const confirmValue = editor.confirmModal.value;
  const rename = editor.renameValue.value;
  const wizard = editor.wizardOpen.value;
  const template = editor.templateOpen.value;
  const picker = editor.pickerOpen.value;

  return (
    <main class="automation-view">
      <PageHeader
        title={t(locale, 'automations')}
        subtitle={t(locale, 'automationsLead')}
        icon={<IconSparkles />}
        action={
          <div class="automation-header-actions">
            <Button variant="ghost" size="sm" onClick={props.onRefresh}>{t(locale, 'refresh')}</Button>
            <Button variant="soft" size="sm" icon={<IconTemplate size={14} />} onClick={editor.requestTemplateWorkflow}>{t(locale, 'templates')}</Button>
            <Button variant="primary" size="sm" onClick={editor.requestCreateWorkflow}>{t(locale, 'newWorkflow')}</Button>
          </div>
        }
      />

      {error ? <Alert variant="danger">{error}</Alert> : null}
      {editor.editorError.value ? <Alert variant="warning">{editor.editorError.value}</Alert> : null}

      <div class="automation-workspace automation-workspace--simple">
        <WorkflowSidebar
          locale={locale}
          workflows={workflows}
          selectedId={editor.selectedId.value}
          nodes={nodes}
          canAddStep={!!draftValue}
          onSelectWorkflow={editor.selectWorkflow}
          onCreateWorkflow={editor.requestCreateWorkflow}
          onAddStep={() => { editor.pickerOpen.value = true; }}
        />

        <section class="automation-editor-panel">
          {!draftValue ? (
            <EmptyState title={t(locale, 'selectWorkflow')} description={t(locale, 'noWorkflows')} action={<Button variant="primary" onClick={editor.requestCreateWorkflow}>{t(locale, 'newWorkflow')}</Button>} />
          ) : (
            <>
              <div class="automation-editor-toolbar">
                <div class="automation-workflow-name">
                  <button type="button" class="automation-workflow-name-button" onClick={() => { editor.renameValue.value = draftValue.name; }}>
                    <span class="automation-workflow-name-button__value">{draftValue.name}</span>
                    <span class="automation-workflow-name-button__edit" aria-hidden="true"><IconEdit size={12} /></span>
                  </button>
                  {dirtyValue ? <Badge tone="pink">{t(locale, 'unsavedChanges')}</Badge> : null}
                </div>
                <div class="automation-toolbar-actions">
                  {selectedRecord ? (
                    <Checkbox checked={draftValue.enabled} onCheckedChange={(enabled) => { editor.draft.value = { ...editor.draft.value!, enabled }; props.onSetEnabled(selectedRecord.id, enabled); }} label={draftValue.enabled ? t(locale, 'disableWorkflow') : t(locale, 'enableWorkflow')} />
                  ) : null}
                  <Button variant="primary" size="sm" disabled={!dirtyValue} onClick={editor.handleSave}>{t(locale, 'saveWorkflow')}</Button>
                  {selectedRecord ? <Button variant="danger" size="sm" icon={<IconTrash />} iconOnly tooltip={t(locale, 'deleteWorkflow')} onClick={editor.handleDeleteWorkflow} /> : null}
                </div>
              </div>

              <WorkflowCanvas
                locale={locale}
                graph={draftValue}
                definitions={nodes}
                selectedNodeId={editor.selectedNodeId.value}
                onSelectNode={(id) => { editor.selectedNodeId.value = id; }}
                onAddNode={() => { editor.pickerOpen.value = true; }}
                onDeleteNode={editor.handleDeleteNode}
              />
            </>
          )}
        </section>

        <NodeInspector
          locale={locale}
          selectedNode={selectedNode}
          selectedDefinition={selectedDefinition}
          onConfigure={editor.openConfiguration}
        />
      </div>

      {wizard ? <WorkflowWizardModal locale={locale} onClose={() => { editor.wizardOpen.value = false; }} onCreate={editor.handleCreateWorkflow} /> : null}
      {template ? <WorkflowTemplateModal locale={locale} definitions={nodes} pluginTemplates={props.pluginTemplates} onClose={() => { editor.templateOpen.value = false; }} onCreate={editor.handleCreateFromTemplate} onOpenMediaPicker={props.onOpenMediaPicker} /> : null}
      {picker ? <NodePickerModal locale={locale} definitions={nodes} onClose={() => { editor.pickerOpen.value = false; }} onSelect={editor.handleAddNode} /> : null}
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
            editor.handleConfigChange(config);
            editor.configuringNodeId.value = null;
          }}
          onAnalyzeScript={props.onAnalyzeScript}
          onOpenMediaPicker={props.onOpenMediaPicker}
          onClose={() => { editor.configuringNodeId.value = null; }}
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
          onConfirm={editor.handleRename}
          onClose={() => { editor.renameValue.value = null; }}
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
            editor.confirmModal.value = null;
            editor.dirty.value = false;
            editor.applyWorkflowAction(pending.action);
          }}
          onClose={() => { editor.confirmModal.value = null; }}
        />
      ) : null}
      {confirmValue?.kind === 'delete' ? (
        <ConfirmModal
          title={t(locale, 'deleteWorkflow')}
          description={t(locale, 'deleteWorkflowConfirm', { name: confirmValue.name })}
          confirmLabel={t(locale, 'deleteWorkflow')}
          cancelLabel={t(locale, 'cancel')}
          danger
          onConfirm={() => editor.confirmDelete(confirmValue.id)}
          onClose={() => { editor.confirmModal.value = null; }}
        />
      ) : null}
    </main>
  );
  };
  },
);

export default AutomationsView;
</script>
