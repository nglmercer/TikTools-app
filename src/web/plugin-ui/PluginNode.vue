<script lang="tsx">
import { computed, createVNode } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import {
  optionFields,
  type PluginConnectionState,
} from '../../automation/plugins/declarative.ts';
import type { JsonObject } from '../../automation/types.ts';
import {
  parseBinding,
  settingsPathSegments,
  type PluginUiAction,
  type PluginUiNode,
} from '../../plugin-ui/index.ts';
import { readSettingsPath, writeSettingsPath } from '../../shared/settings-values.ts';
import { withSchemaDefaults } from '../components/plugin-connection-logic.ts';
import { PluginConnectionCard } from '../components/plugin-connection-card.vue';
import { SchemaForm } from '../components/ui/SchemaForm.vue';
import { i18nText, t, type Locale } from '../i18n.ts';
import { toSettingValues, type PluginUiContext } from './PluginUiContext.ts';

type PluginNodeProps = {
  node: PluginUiNode;
  context: PluginUiContext;
  nodeKey: string;
};



/**
 * Generic recursive renderer for the declarative plugin UI contract.
 *
 * Every node type maps through the registry below — there is no
 * domain-specific switch over plugin kinds, and this module never imports
 * TTS (or any other domain) UI. Domain node types (`tts-settings`, …)
 * resolve through `context.customNodes`, which the composition root binds
 * to host-owned components. Manifest data is rendered as text and form
 * controls only: no v-html, no innerHTML, no eval, no dynamic components
 * resolved from manifest strings.
 */
export const PluginNode = defineVueComponent<PluginNodeProps>(
  ['node', 'context', 'nodeKey'],
  (props) => {
    const dynamicFields = computed(() => {
      if (props.node.type !== 'form') return [];
      const node = props.node;
      return optionFields(node.uiHints ?? props.context.settings.state?.uiHints);
    });

    const fieldOptions = computed(() => {
      const merged: Record<string, Array<{ value: string; label: string }>> = {};
      for (const field of dynamicFields.value) {
        const options = props.context.options.values[field.source];
        if (options && options.length > 0) merged[field.key] = options;
      }
      return merged;
    });

    const bindingValue = (bind: string): string | number | boolean | undefined => {
      const parsed = parseBinding(bind);
      if (!parsed) return undefined;
      if (parsed.scope === 'local') return props.context.local.get(parsed.path);
      if (parsed.scope === 'source') {
        // Read-only convenience: the selected value of the option source
        // whose field segment matches, e.g. `source.voice`.
        for (const [source, selected] of Object.entries(props.context.options.selected)) {
          if (source.split(':').pop() === parsed.path) return selected;
        }
        return undefined;
      }
      const value = readSettingsPath(
        props.context.settings.state?.values,
        settingsPathSegments(parsed),
      );
      return typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean'
        ? value
        : undefined;
    };

    const writeBinding = (bind: string, value: string | number | boolean): void => {
      const parsed = parseBinding(bind);
      if (!parsed || parsed.scope === 'source') return;
      if (parsed.scope === 'local') {
        props.context.local.set(parsed.path, value);
        return;
      }
      const current = props.context.settings.state?.values ?? {};
      const next = writeSettingsPath(current, settingsPathSegments(parsed), value);
      props.context.settings.save(toSettingValues(next));
    };

    const runAction = (action: PluginUiAction): void => {
      const context = props.context;
      switch (action.type) {
        case 'save-settings': {
          // Merges every form draft on the page (in key order) over the
          // current host values. Single-form pages — all current pages —
          // save exactly that form's draft.
          const merged: JsonObject = { ...(context.settings.state?.values ?? {}) };
          for (const key of [...context.formDrafts.keys()].sort()) {
            Object.assign(merged, context.formDrafts.get(key) ?? {});
          }
          context.settings.save(toSettingValues(merged));
          break;
        }
        case 'plugin-action':
          context.actions.execute(action.actionType, toSettingValues(action.config ?? {}));
          break;
        case 'refresh-source':
          context.options.get(action.source, true);
          break;
        case 'test-connection':
          context.connection.test();
          break;
        case 'open-media-picker': {
          const pick = context.media.pick;
          if (!pick) break;
          // `accept` is contract-reserved for host kind filtering; the
          // current picker takes only a title, so selection is unfiltered.
          void action.accept;
          pick({ title: context.pluginName }, (selection, error) => {
            if (error || !selection || selection.type !== 'file' || !action.target) return;
            const parsed = parseBinding(action.target);
            if (!parsed || parsed.scope !== 'settings') return;
            const current = context.settings.state?.values ?? {};
            const next = writeSettingsPath(
              current,
              settingsPathSegments(parsed),
              selection.file.path,
            );
            context.settings.save(toSettingValues(next));
          });
          break;
        }
      }
    };

    const renderForm = (nodeKey: string) => {
      const node = props.node;
      if (node.type !== 'form') return null;
      const context = props.context;
      const locale: Locale = context.locale;
      const state = context.settings.state;
      const schema = node.schema ?? state?.schema;
      if (!schema) return null;
      const formValues = withSchemaDefaults(
        context.formDrafts.get(nodeKey) ?? state?.values ?? {},
        schema,
      );
      const title = node.title ? i18nText(locale, node.title) : '';
      return (
        <section class="plg-stack">
          {title && <h3 class="plg-topbar__title">{title}</h3>}
          <div class="plg-form">
            <SchemaForm
              locale={locale}
              schema={schema}
              uiHints={node.uiHints ?? state?.uiHints}
              value={formValues}
              fieldOptions={fieldOptions.value}
              onChange={(next) => {
                context.formDrafts.set(nodeKey, next);
              }}
              onOpenMediaPicker={context.media.pick}
            />
            <div class="plg-row">
              <button
                type="button"
                class="plg-btn plg-btn--primary plg-btn--sm"
                disabled={!state}
                onClick={() => {
                  const values = context.formDrafts.get(nodeKey) ?? state?.values;
                  if (values) context.settings.save(toSettingValues(values));
                }}
              >
                {t(locale, 'pluginSettingsSave')}
              </button>
            </div>
          </div>
        </section>
      );
    };

    const renderConnection = () => {
      const context = props.context;
      const connection: PluginConnectionState | undefined = context.connection.state;
      return (
        <PluginConnectionCard
          locale={context.locale}
          pluginId={context.pluginId}
          pluginName={context.pluginName}
          settingsState={context.settings.state}
          connection={connection}
          actionOptions={context.options.values}
          onGetSettings={() => context.settings.get()}
          onSaveSettings={(_id, values) => context.settings.save(values)}
          onGetActionOptions={(source) => context.options.get(source)}
          onTestConnection={() => context.connection.test()}
          onOpenMediaPicker={context.media.pick}
          supportsProvisioning={context.provisioning.supported}
          provisionState={context.provisioning.state}
          onProvisionToken={(_id, username, password) =>
            context.provisioning.provision(username, password)
          }
        />
      );
    };

    const renderList = () => {
      const node = props.node;
      if (node.type !== 'list') return null;
      const context = props.context;
      const locale: Locale = context.locale;
      const title = node.title ? i18nText(locale, node.title) : '';
      const options = context.options.values[node.optionsFrom] ?? [];
      const listError = context.options.errors[node.optionsFrom];
      return (
        <section class="plg-stack">
          {title && <h3 class="plg-topbar__title">{title}</h3>}
          {listError && (
            <div class="plg-alert" role="status">
              {listError}
            </div>
          )}
          {options.length > 0 ? (
            <ul class="plg-list">
              {options.map((option) => (
                <li key={option.value} class="plg-list__row">
                  <span class="plg-list__label">{option.label}</span>
                  <span class="plg-pill plg-pill--mono">{option.value}</span>
                </li>
              ))}
            </ul>
          ) : (
            !listError && <span class="plg-group-note">{t(locale, 'pluginListEmpty')}</span>
          )}
          <div class="plg-row">
            <button
              type="button"
              class="plg-btn plg-btn--sm"
              onClick={() => context.options.get(node.optionsFrom, true)}
            >
              {t(locale, 'pluginListRefresh')}
            </button>
          </div>
        </section>
      );
    };

    const renderSelect = () => {
      const node = props.node;
      if (node.type !== 'select') return null;
      const context = props.context;
      const locale: Locale = context.locale;
      const label = node.label ? i18nText(locale, node.label) : '';
      const sourceOptions =
        node.optionsFrom != null ? (context.options.values[node.optionsFrom] ?? []) : [];
      const rows =
        sourceOptions.length > 0
          ? sourceOptions
          : ((node.options ?? []).map((option) => ({
              value: option.value,
              label: i18nText(locale, option.label),
            })) as Array<{ value: string; label: string }>);
      const current = bindingValue(node.bind);
      const id = `plg-${props.nodeKey.replace(/[^a-zA-Z0-9_-]/g, '-')}`;
      return (
        <div class="tts-row tts-row--stack">
          {label && (
            <label class="tts-label" for={id}>
              {label}
            </label>
          )}
          <select
            id={id}
            class="tts-select"
            value={typeof current === 'string' ? current : ''}
            onChange={(event) =>
              writeBinding(node.bind, (event.currentTarget as HTMLSelectElement).value)
            }
          >
            {rows.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>
      );
    };

    const renderRange = () => {
      const node = props.node;
      if (node.type !== 'range') return null;
      const locale: Locale = props.context.locale;
      const label = node.label ? i18nText(locale, node.label) : '';
      const current = bindingValue(node.bind);
      const numeric = typeof current === 'number' ? current : (node.min ?? 0);
      const id = `plg-${props.nodeKey.replace(/[^a-zA-Z0-9_-]/g, '-')}`;
      return (
        <div class="tts-row tts-row--stack">
          {label && (
            <label class="tts-label" for={id}>
              {label} <span class="tts-value">{numeric}</span>
            </label>
          )}
          <input
            id={id}
            class="tts-range"
            type="range"
            min={node.min ?? 0}
            max={node.max ?? 1}
            step={node.step ?? 0.01}
            value={numeric}
            onInput={(event) =>
              writeBinding(node.bind, Number((event.currentTarget as HTMLInputElement).value))
            }
          />
        </div>
      );
    };

    const renderCheckbox = () => {
      const node = props.node;
      if (node.type !== 'checkbox') return null;
      const locale: Locale = props.context.locale;
      const label = node.label ? i18nText(locale, node.label) : '';
      const current = bindingValue(node.bind);
      return (
        <label class="tts-check">
          <input
            type="checkbox"
            checked={current === true}
            onChange={(event) =>
              writeBinding(node.bind, (event.currentTarget as HTMLInputElement).checked)
            }
          />
          {label}
        </label>
      );
    };

    const renderButton = () => {
      const node = props.node;
      if (node.type !== 'button') return null;
      const locale: Locale = props.context.locale;
      const label = node.label
        ? i18nText(locale, node.label)
        : node.title
          ? i18nText(locale, node.title)
          : '';
      const classes = ['plg-btn', 'plg-btn--sm'];
      if (node.variant === 'primary') classes.push('plg-btn--primary');
      if (node.variant === 'danger') classes.push('plg-btn--danger');
      return (
        <div class="plg-row">
          <button type="button" class={classes.join(' ')} onClick={() => runAction(node.action)}>
            {label}
          </button>
        </div>
      );
    };

    const renderText = () => {
      const node = props.node;
      if (node.type !== 'text') return null;
      const locale: Locale = props.context.locale;
      const title = node.title ? i18nText(locale, node.title) : '';
      return (
        <section class="plg-stack">
          {title && <h3 class="plg-topbar__title">{title}</h3>}
          <p class="plg-plugin__desc">{i18nText(locale, node.text)}</p>
        </section>
      );
    };

    const renderStatus = () => {
      const node = props.node;
      if (node.type !== 'status') return null;
      const locale: Locale = props.context.locale;
      return (
        <div class="plg-alert" role="status">
          {i18nText(locale, node.text)}
        </div>
      );
    };

    // Renderer registry: one entry per generic node type. Domain node types
    // fall through to `customNodes` (host-injected, never manifest-resolved).
    const renderers: Record<string, (nodeKey: string) => unknown> = {
      text: () => renderText(),
      status: () => renderStatus(),
      separator: () => <hr class="plg-separator" />,
      form: (nodeKey) => renderForm(nodeKey),
      connection: () => renderConnection(),
      list: () => renderList(),
      select: () => renderSelect(),
      range: () => renderRange(),
      checkbox: () => renderCheckbox(),
      button: () => renderButton(),
      stack: (nodeKey) => {
        const node = props.node;
        if (node.type !== 'stack') return null;
        return (
          <div class="plg-stack">
            {node.children.map((child, index) => (
              <PluginNode
                key={index}
                node={child}
                context={props.context}
                nodeKey={`${nodeKey}/${index}`}
              />
            ))}
          </div>
        );
      },
      card: (nodeKey) => {
        const node = props.node;
        if (node.type !== 'card') return null;
        const locale: Locale = props.context.locale;
        const title = node.title ? i18nText(locale, node.title) : '';
        return (
          <section class="tts-card">
            {title && <h4 class="tts-card__title">{title}</h4>}
            {node.children.map((child, index) => (
              <PluginNode
                key={index}
                node={child}
                context={props.context}
                nodeKey={`${nodeKey}/${index}`}
              />
            ))}
          </section>
        );
      },
    };

    return () => {
      const node = props.node;
      const render = renderers[node.type];
      if (render) return render(props.nodeKey) as never;
      // Domain nodes (tts-settings, …): host-injected renderers only. An
      // unregistered domain node renders a neutral note — never a crash,
      // never manifest-resolved code.
      const Custom = props.context.customNodes[node.type];
      if (!Custom) {
        return (
          <section class="plg-stack">
            <span class="plg-group-note">{t(props.context.locale, 'pluginListEmpty')}</span>
          </section>
        );
      }
      return createVNode(Custom as never, {
        node,
        context: props.context,
        nodeKey: props.nodeKey,
      });
    };
  },
);

export default PluginNode;
</script>
