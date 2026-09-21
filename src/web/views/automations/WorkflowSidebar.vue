<script lang="tsx">
import { defineVueComponent } from '../../vue/component.ts';

import type { NodeDefinition } from '../../../automation/types.ts';
import type { AutomationWorkflowRecord } from '../../../shared/messages.ts';
import { IconPlus } from '../../components/icons.vue';
import { Badge, EmptyState } from '../../components/ui/Card.vue';
import { Button } from '../../components/ui/Button.vue';
import { t, type Locale } from '../../i18n.ts';

type WorkflowSidebarProps = {
  locale: Locale;
  workflows: AutomationWorkflowRecord[];
  selectedId: string | null;
  nodes: NodeDefinition[];
  canAddStep: boolean;
  onSelectWorkflow: (record: AutomationWorkflowRecord) => void;
  onCreateWorkflow: () => void;
  onAddStep: () => void;
};

/** Workflow list plus node-catalog shortcuts beside the canvas. */
export const WorkflowSidebar = defineVueComponent<WorkflowSidebarProps>(
  ['locale', 'workflows', 'selectedId', 'nodes', 'canAddStep', 'onSelectWorkflow', 'onCreateWorkflow', 'onAddStep'],
  (props) => {
  return () => {
  const locale = props.locale;
  const workflows = props.workflows;
  const nodes = props.nodes;
  return (
    <aside class="automation-sidebar">
      <div class="automation-panel-heading">
        <span>{t(locale, 'automations')}</span>
        <Badge tone="cyan">{workflows.length}</Badge>
      </div>

      {workflows.length === 0 ? (
        <EmptyState title={t(locale, 'noWorkflows')} action={<Button variant="soft" size="sm" onClick={props.onCreateWorkflow}>{t(locale, 'newWorkflow')}</Button>} />
      ) : (
        <div class="automation-workflow-list">
          {workflows.map((record) => (
            <button key={record.id} type="button" class={`automation-workflow-item ${props.selectedId === record.id ? 'is-active' : ''}`} onClick={() => props.onSelectWorkflow(record)}>
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
        <Button variant="cyan" block disabled={!props.canAddStep} icon={<IconPlus size={14} />} onClick={props.onAddStep}>{t(locale, 'addStep')}</Button>
        <p class="automation-panel-hint automation-panel-hint--secondary">{t(locale, 'automationFlowHint')}</p>
      </div>
    </aside>
  );
  };
  },
);

export default WorkflowSidebar;
</script>
