<script setup lang="ts">
import { computed, ref, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JsonObject } from '../../../../automation/types.ts';
import type { PluginUiNode } from '../../../../plugin-ui/index.ts';
import { toSettingValues } from '../../../../shared/settings-values.ts';
import { fieldOptionsFor } from '@/entities/plugin/lib/plugin-options.ts';
import { readBinding, runAction, writeBinding, type BindingValue } from '@/entities/plugin/lib/plugin-ui-runtime.ts';
import type { PluginUiContext } from '@/entities/plugin/model/plugin-ui-context.ts';
import type { PluginActionOutcome } from '@/entities/plugin/model/plugins-state.ts';
import PluginConnectionEditor from '@/features/configure-plugin-connection/ui/PluginConnectionEditor.vue';
import PluginActionStatus from './PluginActionStatus.vue';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseSchemaForm from '@/shared/ui/schema-form/BaseSchemaForm.vue';
import { withSchemaDefaults } from '@/shared/ui/schema-form/lib/schema-fields.ts';
import BaseSlider from '@/shared/ui/slider/BaseSlider.vue';

defineOptions({ name: 'PluginUiNode' });

const props = defineProps<{
  node: PluginUiNode;
  context: PluginUiContext;
  nodeKey: string;
}>();

const { t } = useI18n();
const id = useId();

type ActionFeedback = { busy: boolean; outcome?: PluginActionOutcome };
const actionFeedback = ref<Record<string, ActionFeedback>>({});

const title = computed(() => {
  const node = props.node;
  return node.title ? localizedText(node.title) : node.label ? localizedText(node.label) : '';
});
const custom = computed(() => props.context.customNodes[(props.node as { type: string }).type]);
const label = computed(() => (props.node.label ? localizedText(props.node.label) : ''));
const children = computed(() => (props.node.type === 'stack' || props.node.type === 'card' ? props.node.children : []));
type ChildEntry = { node: PluginUiNode; index: number };
type ChildItem = { key: string; buttons: boolean; node: ChildEntry; entries: ChildEntry[] };
const childItems = computed<ChildItem[]>(() => {
  const items: ChildItem[] = [];
  let index = 0;
  while (index < children.value.length) {
    const child = children.value[index];
    if (!child) break;
    if (child.type !== 'button') {
      items.push({ key: `node-${index}`, buttons: false, node: { node: child, index }, entries: [{ node: child, index }] });
      index += 1;
      continue;
    }
    const first = { node: child, index };
    const entries: ChildEntry[] = [first];
    const start = index;
    index += 1;
    while (index < children.value.length) {
      const button = children.value[index];
      if (!button || button.type !== 'button') break;
      entries.push({ node: button, index });
      index += 1;
    }
    items.push({ key: `buttons-${start}-${index}`, buttons: true, node: first, entries });
  }
  return items;
});
const bound = computed<BindingValue | undefined>(() => ('bind' in props.node ? readBinding(props.context, props.node.bind) : undefined));

const formSchema = computed(() => (props.node.type === 'form' ? (props.node.schema ?? props.context.settings.state?.schema) : undefined));
const formHints = computed(() => (props.node.type === 'form' ? (props.node.uiHints ?? props.context.settings.state?.uiHints) : undefined));
const formValues = computed<JsonObject>({
  get: () =>
    withSchemaDefaults(props.context.formDrafts.get(props.nodeKey) ?? props.context.settings.state?.values ?? {}, formSchema.value),
  set: (next) => props.context.formDrafts.set(props.nodeKey, next),
});
const fieldOptions = computed(() => fieldOptionsFor(formHints.value, props.context.options.values));

const selectOptions = computed(() => {
  const node = props.node;
  if (node.type !== 'select') return [];
  const fromSource = node.optionsFrom ? (props.context.options.values[node.optionsFrom] ?? []) : [];
  if (fromSource.length) return fromSource.map((option) => ({ value: option.value, label: option.label }));
  return (node.options ?? []).map((option) => ({ value: option.value, label: localizedText(option.label) }));
});
const listOptions = computed(() => (props.node.type === 'list' ? (props.context.options.values[props.node.optionsFrom] ?? []) : []));
const listError = computed(() => (props.node.type === 'list' ? props.context.options.errors[props.node.optionsFrom] : undefined));
const rangeValue = computed(() => {
  if (props.node.type !== 'range') return 0;
  return typeof bound.value === 'number' ? bound.value : (props.node.min ?? 0);
});

function write(value: BindingValue | undefined): void {
  if (value !== undefined && 'bind' in props.node) writeBinding(props.context, props.node.bind, value);
}

function saveForm(): void {
  const values = props.context.formDrafts.get(props.nodeKey) ?? props.context.settings.state?.values;
  if (values) props.context.settings.save(toSettingValues(values));
}

function actionKey(node: PluginUiNode): string {
  if (node.type !== 'button') return '';
  return node.action.type === 'plugin-action' ? node.action.actionType : node.action.type;
}

function feedbackFor(node: PluginUiNode): ActionFeedback | undefined {
  return actionFeedback.value[actionKey(node)];
}

function isActionDisabled(node: PluginUiNode): boolean {
  if (node.type !== 'button') return false;
  if (feedbackFor(node)?.busy) return true;
  if (!node.disabledWhen) return false;
  const value = readBinding(props.context, node.disabledWhen.bind);
  return typeof value === 'string' && node.disabledWhen.values.includes(value);
}

async function pressNode(node: PluginUiNode): Promise<void> {
  if (node.type !== 'button') return;
  const key = actionKey(node);
  if (actionFeedback.value[key]?.busy) return;
  actionFeedback.value = { ...actionFeedback.value, [key]: { busy: true } };
  try {
    const outcome = await runAction(props.context, node.action);
    actionFeedback.value = { ...actionFeedback.value, [key]: { busy: false, outcome: outcome ?? undefined } };
  } catch {
    actionFeedback.value = { ...actionFeedback.value, [key]: { busy: false } };
  }
}

function press(): void {
  void pressNode(props.node);
}

function feedbackText(node: PluginUiNode): string {
  const feedback = feedbackFor(node);
  if (!feedback) return '';
  if (feedback.busy) return t('plugins.page.working');
  return feedback.outcome?.error ?? feedback.outcome?.summary ?? '';
}

function actionLabel(node: PluginUiNode): string {
  if (node.type !== 'button') return '';
  return node.title ? localizedText(node.title) : node.label ? localizedText(node.label) : '';
}
</script>

<template>
  <div
    v-if="node.type === 'stack'"
    class="plugin-node__stack"
  >
    <template
      v-for="item in childItems"
      :key="item.key"
    >
      <div
        v-if="item.buttons"
        class="plugin-node__actions"
      >
        <template
          v-for="entry in item.entries"
          :key="entry.index"
        >
          <BaseButton
            size="sm"
            :variant="entry.node.type === 'button' ? (entry.node.variant ?? 'default') : 'default'"
            :disabled="isActionDisabled(entry.node)"
            @click="pressNode(entry.node)"
          >
            {{ actionLabel(entry.node) }}
          </BaseButton>
          <span
            v-if="feedbackText(entry.node)"
            class="plugin-node__action-feedback"
            :class="{ 'plugin-node__action-feedback--error': feedbackFor(entry.node)?.outcome?.ok === false }"
            role="status"
          >{{ feedbackText(entry.node) }}</span>
        </template>
      </div>
      <PluginUiNode
        v-else
        :node="item.node.node"
        :context="context"
        :node-key="`${nodeKey}/${item.node.index}`"
      />
    </template>
  </div>
  <BaseGroupBox
    v-else-if="node.type === 'card'"
    :title="title"
  >
    <!-- A card is a titled stack: draw its children through the stack branch instead of repeating it. -->
    <PluginUiNode
      :node="{ type: 'stack', children: node.children }"
      :context="context"
      :node-key="nodeKey"
    />
  </BaseGroupBox>
  <section
    v-else-if="node.type === 'text'"
    class="plugin-node__text"
  >
    <h3 v-if="title">
      {{ title }}
    </h3>
    <p>{{ localizedText(node.text) }}</p>
  </section>
  <p
    v-else-if="node.type === 'status'"
    class="plugin-node__status"
    :class="`plugin-node__status--${node.tone ?? 'info'}`"
    role="status"
  >
    {{ localizedText(node.text) }}
  </p>
  <hr
    v-else-if="node.type === 'separator'"
    class="plugin-node__separator"
  >
  <section
    v-else-if="node.type === 'form'"
    class="plugin-node__form"
  >
    <h3 v-if="title">
      {{ title }}
    </h3>
    <template v-if="formSchema">
      <BaseSchemaForm
        v-model="formValues"
        :schema="formSchema"
        :ui-hints="formHints"
        :field-options="fieldOptions"
        :open-media-picker="context.media.pick"
      />
      <div class="plugin-node__actions">
        <BaseButton
          variant="primary"
          size="sm"
          :disabled="!context.settings.state"
          @click="saveForm"
        >
          {{ t('plugins.page.save') }}
        </BaseButton>
      </div>
    </template>
  </section>
  <PluginConnectionEditor
    v-else-if="node.type === 'connection'"
    :plugin-id="context.pluginId"
    :plugin-name="context.pluginName"
    :settings="context.settings.state"
    :connection="context.connection.state"
    :action-options="context.options.values"
    :open-media-picker="context.media.pick"
    :supports-token-import="context.provisioning.supportsImport"
    :supports-token-discovery="context.provisioning.supportsDiscovery"
    :supports-provisioning="context.provisioning.supported"
    :provision-state="context.provisioning.state"
    @load-settings="context.settings.get()"
    @load-options="context.options.get($event)"
    @save="(_id, values) => context.settings.save(values)"
    @test="context.connection.test()"
    @import-token="(_id, options) => context.provisioning.importToken(options)"
    @provision="(_id, username, password) => context.provisioning.provision(username, password)"
  />
  <PluginActionStatus
    v-else-if="node.type === 'action-status'"
    :node="node"
    :context="context"
  />
  <section
    v-else-if="node.type === 'list'"
    class="plugin-node__list"
  >
    <h3 v-if="title">
      {{ title }}
    </h3>
    <p
      v-if="listError"
      class="plugin-node__status plugin-node__status--error"
      role="status"
    >
      {{ listError }}
    </p>
    <ul v-if="listOptions.length">
      <li
        v-for="option in listOptions"
        :key="option.value"
      >
        <span>{{ option.label }}</span>
        <code>{{ option.value }}</code>
      </li>
    </ul>
    <p
      v-else-if="!listError"
      class="plugin-node__muted"
    >
      {{ t('plugins.page.listEmpty') }}
    </p>
    <div class="plugin-node__actions">
      <BaseButton
        size="sm"
        @click="context.options.get(node.optionsFrom, true)"
      >
        {{ t('plugins.page.refresh') }}
      </BaseButton>
    </div>
  </section>
  <div
    v-else-if="node.type === 'select'"
    class="plugin-node__control"
  >
    <label
      v-if="label"
      :for="id"
    >{{ label }}</label>
    <BaseComboBox
      :id="id"
      :model-value="typeof bound === 'string' ? bound : undefined"
      :options="selectOptions"
      @update:model-value="write($event)"
    />
  </div>
  <div
    v-else-if="node.type === 'range'"
    class="plugin-node__control"
  >
    <label
      v-if="label"
      :for="id"
    >{{ label }} <output>{{ rangeValue }}</output></label>
    <BaseSlider
      :id="id"
      :model-value="rangeValue"
      :min="node.min ?? 0"
      :max="node.max ?? 1"
      :step="node.step ?? 0.01"
      @update:model-value="write($event)"
    />
  </div>
  <BaseCheckbox
    v-else-if="node.type === 'checkbox'"
    :model-value="bound === true"
    @update:model-value="write($event)"
  >
    {{ label }}
  </BaseCheckbox>
  <div
    v-else-if="node.type === 'button'"
    class="plugin-node__actions"
  >
    <BaseButton
      size="sm"
      :variant="node.variant ?? 'default'"
      :disabled="isActionDisabled(node)"
      @click="press"
    >
      {{ title }}
    </BaseButton>
    <span
      v-if="feedbackText(node)"
      class="plugin-node__action-feedback"
      :class="{ 'plugin-node__action-feedback--error': feedbackFor(node)?.outcome?.ok === false }"
      role="status"
    >{{ feedbackText(node) }}</span>
  </div>
  <component
    :is="custom"
    v-else-if="custom"
    :node="node"
    :context="context"
    :node-key="nodeKey"
  />
  <p
    v-else
    class="plugin-node__muted"
  >
    {{ t('plugins.page.unsupported') }}
  </p>
</template>

<style scoped>
.plugin-node__stack,
.plugin-node__form,
.plugin-node__list {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.plugin-node__text h3,
.plugin-node__form h3,
.plugin-node__list h3 {
  margin: 0;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-semibold);
}

.plugin-node__text p {
  margin: var(--space-2) 0 0;
  color: var(--color-text-muted);
}

.plugin-node__status {
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
}

.plugin-node__status--ok {
  border-color: var(--color-success);
  color: var(--color-success);
}

.plugin-node__status--error {
  border-color: var(--color-danger);
  color: var(--color-danger-hover);
}

.plugin-node__separator {
  width: 100%;
  margin: 0;
  border: 0;
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.plugin-node__list ul {
  display: flex;
  flex-direction: column;
  margin: 0;
  padding: 0;
  border: var(--border-width) solid var(--color-border-subtle);
  list-style: none;
}

.plugin-node__list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-2) var(--space-4);
}

.plugin-node__list li + li {
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.plugin-node__list code {
  color: var(--color-text-subtle);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}

.plugin-node__control {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  max-width: 420px;
}

.plugin-node__control label {
  color: var(--color-text-muted);
}

.plugin-node__actions {
  display: flex;
  align-items: center;
  align-self: flex-start;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.plugin-node__muted {
  margin: 0;
  color: var(--color-text-subtle);
}
</style>
