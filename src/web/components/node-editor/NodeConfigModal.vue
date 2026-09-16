<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import type {
  AutomationEvent,
  AutomationEventType,
  AutomationScriptAnalysis,
  JsonObject,
  NodeDefinition,
  WorkflowNode,
} from '../../../automation/types.ts';
import { Button } from '../ui/Button.vue';
import { Modal } from '../ui/Modal.vue';
import { NodeConfigForm } from './NodeConfigForm.vue';
import { EventContextPreview } from './EventContextPreview.vue';
import { t, type Locale } from '../../i18n.ts';
import type { OpenMediaPicker } from '../../../shared/messages.ts';

type NodeConfigModalProps = {
  locale: Locale;
  node: WorkflowNode;
  definition?: NodeDefinition;
  eventType?: AutomationEventType;
  lastEvent?: AutomationEvent;
  lastEventCapturedAt?: number;
  analysis?: AutomationScriptAnalysis;
  onApply: (config: JsonObject) => void;
  onAnalyzeScript: (nodeId: string, source: string, offset: number, eventType?: AutomationEventType) => void;
  onOpenMediaPicker?: OpenMediaPicker;
  onClose: () => void;
};

/**
 * Edits a local copy of a node. The workflow draft is changed only after the
 * user confirms, so closing the modal is a safe cancel operation.
 */
export const NodeConfigModal = defineVueComponent<NodeConfigModalProps>(
  ['locale', 'node', 'definition', 'eventType', 'lastEvent', 'lastEventCapturedAt', 'analysis', 'onApply', 'onAnalyzeScript', 'onOpenMediaPicker', 'onClose'],
  (props) => {
  const config = ref<JsonObject>({ ...props.node.config });
  return () => {
    const { locale, node, definition, eventType, lastEvent, lastEventCapturedAt, analysis, onApply, onAnalyzeScript, onOpenMediaPicker, onClose } = props;
    const title = definition?.title ?? node.type;
    const draftNode: WorkflowNode = { ...node, config: config.value };
    return (
    <Modal
      title={`${t(locale, 'configureStep')}: ${title}`}
      description={t(locale, 'configureStepHint')}
      onClose={onClose}
      size="lg"
      footer={
        <div class="node-editor-modal-actions">
          <Button variant="soft" onClick={onClose}>{t(locale, 'cancel')}</Button>
          <Button variant="primary" onClick={() => onApply(config.value)}>{t(locale, 'applyChanges')}</Button>
        </div>
      }
    >
      <div class="node-editor-config-modal__body">
        <EventContextPreview locale={locale} event={lastEvent} capturedAt={lastEventCapturedAt} />
        <NodeConfigForm
          locale={locale}
          node={draftNode}
          definition={definition}
          eventType={eventType}
          lastEvent={lastEvent}
          analysis={analysis}
          onChange={(next) => (config.value = next)}
          onAnalyzeScript={onAnalyzeScript}
          onOpenMediaPicker={onOpenMediaPicker}
        />
      </div>
    </Modal>
    );
  };
  },
);

export default NodeConfigModal;
</script>
