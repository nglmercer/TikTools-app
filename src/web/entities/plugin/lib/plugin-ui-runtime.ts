import type { JsonObject } from '../../../../automation/types.ts';
import { parseBinding, settingsPathSegments, type PluginUiAction, type PluginUiNode } from '../../../../plugin-ui/index.ts';
import { readSettingsPath, toSettingValues, writeSettingsPath } from '../../../../shared/settings-values.ts';
import type { PluginUiContext } from '../model/plugin-ui-context.ts';
import type { PluginActionOutcome } from '../model/plugins-state.ts';

export type BindingValue = string | number | boolean;

export function readBinding(context: PluginUiContext, bind: string): BindingValue | undefined {
  const parsed = parseBinding(bind);
  if (!parsed) return undefined;
  if (parsed.scope === 'local') return context.local.get(parsed.path);
  if (parsed.scope === 'source') {
    for (const [source, selected] of Object.entries(context.options.selected)) {
      if (source.split(':').pop() === parsed.path) return selected;
    }
    return undefined;
  }
  const value = readSettingsPath(context.settings.state?.values, settingsPathSegments(parsed));
  return typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean' ? value : undefined;
}

export function writeBinding(context: PluginUiContext, bind: string, value: BindingValue): void {
  const parsed = parseBinding(bind);
  if (!parsed || parsed.scope === 'source') return;
  if (parsed.scope === 'local') {
    context.local.set(parsed.path, value);
    return;
  }
  const next = writeSettingsPath(context.settings.state?.values ?? {}, settingsPathSegments(parsed), value);
  context.settings.save(toSettingValues(next));
}

export function runAction(context: PluginUiContext, action: PluginUiAction): void | Promise<PluginActionOutcome> {
  switch (action.type) {
    case 'save-settings': {
      const merged: JsonObject = { ...(context.settings.state?.values ?? {}) };
      for (const key of [...context.formDrafts.keys()].sort()) Object.assign(merged, context.formDrafts.get(key) ?? {});
      context.settings.save(toSettingValues(merged));
      return;
    }
    case 'plugin-action':
      return context.actions.execute(action.actionType, toSettingValues(action.config ?? {}));
    case 'refresh-source':
      context.options.get(action.source, true);
      return;
    case 'test-connection':
      context.connection.test();
      return;
    case 'open-media-picker': {
      const target = action.target ? parseBinding(action.target) : undefined;
      if (!context.media.pick || !target || target.scope !== 'settings') return;
      context.media.pick({ title: context.pluginName }, (selection, error) => {
        if (error || selection?.type !== 'file') return;
        const next = writeSettingsPath(context.settings.state?.values ?? {}, settingsPathSegments(target), selection.file.path);
        context.settings.save(toSettingValues(next));
      });
    }
  }
}

export function formNodes(node: PluginUiNode): PluginUiNode[] {
  if (node.type === 'stack' || node.type === 'card') return node.children.flatMap(formNodes);
  return node.type === 'form' ? [node] : [];
}

export function childNodes(node: PluginUiNode): PluginUiNode[] {
  return node.type === 'stack' || node.type === 'card' ? node.children : [node];
}
