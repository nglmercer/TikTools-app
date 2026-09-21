<script lang="tsx">
import { defineVueComponent } from '../../vue/component.ts';

import type { NodeDefinition, WorkflowNode } from '../../../automation/types.ts';
import { Badge } from '../../components/ui/Card.vue';
import { Button } from '../../components/ui/Button.vue';
import { t, type Locale } from '../../i18n.ts';

type NodeInspectorProps = {
  locale: Locale;
  selectedNode?: WorkflowNode;
  selectedDefinition?: NodeDefinition;
  onConfigure: () => void;
};

/** Read-only summary of the selected canvas node with a configure shortcut. */
export const NodeInspector = defineVueComponent<NodeInspectorProps>(
  ['locale', 'selectedNode', 'selectedDefinition', 'onConfigure'],
  (props) => {
  return () => {
  const locale = props.locale;
  const selectedNode = props.selectedNode;
  const selectedDefinition = props.selectedDefinition;
  return (
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
          <Button variant="primary" block onClick={props.onConfigure}>{t(locale, 'configureStep')}</Button>
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
  );
  };
  },
);

export default NodeInspector;
</script>
