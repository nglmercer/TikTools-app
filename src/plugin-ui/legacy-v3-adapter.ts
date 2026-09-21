/**
 * Backward-compatible adapter: legacy schema-v3 pages → generic UI contract.
 *
 * Existing plugin manifests declare pages with the closed v3 section kinds
 * (`text`, `form`, `connection`, `list`, `tts`). This adapter translates
 * each section into generic nodes so the renderer only speaks the
 * declarative contract:
 *
 * - `text` → `text` node.
 * - `form` → `form` node (same schema/uiHints fallback semantics).
 * - `connection` → `connection` node.
 * - `list` → `list` node (refresh is built into the renderer).
 * - `tts` → `status` placeholder. Domain panels left the main frontend:
 *   plugins render their own audio UI in an isolated view; the legacy
 *   marker degrades to a neutral note instead of silently vanishing.
 *
 * This module is the only place in `src/plugin-ui/` that imports
 * automation (the v3 descriptor types plus the option-source normalizer).
 */

import type {
  PluginPageDescriptor,
  PluginPageSection,
} from '../automation/behavior/types.ts';
import { normalizeOptionsFrom } from '../automation/plugins/declarative.ts';
import type { PluginUiNode, PluginUiPage } from './contracts.ts';

function adaptSection(section: PluginPageSection): PluginUiNode[] {
  switch (section.kind) {
    case 'text': {
      if (!section.text) return [];
      const node: PluginUiNode = { type: 'text', text: section.text };
      if (section.title) node.title = section.title;
      return [node];
    }
    case 'form': {
      const node: PluginUiNode = { type: 'form' };
      if (section.schema) node.schema = section.schema;
      if (section.uiHints) node.uiHints = section.uiHints;
      if (section.title) node.title = section.title;
      return [node];
    }
    case 'connection': {
      const node: PluginUiNode = { type: 'connection' };
      if (section.title) node.title = section.title;
      return [node];
    }
    case 'list': {
      if (!section.optionsFrom) return [];
      const source = normalizeOptionsFrom(section.optionsFrom);
      if (!source) return [];
      const list: PluginUiNode = { type: 'list', optionsFrom: source };
      if (section.title) list.title = section.title;
      return [list];
    }
    case 'tts': {
      // Domain UI lives in the plugin's isolated view now. The generic
      // renderer knows no audio concepts, so the legacy marker becomes a
      // neutral status note (default text; hosts may localize the key).
      const node: PluginUiNode = {
        type: 'status',
        tone: 'info',
        text: {
          default: 'This panel moved to the plugin view.',
          i18key: 'pluginLegacyPanelMoved',
        },
      };
      if (section.title) node.title = section.title;
      return [node];
    }
  }
}

/** Adapts one legacy v3 page descriptor to the generic UI contract. */
export function adaptLegacyPage(page: PluginPageDescriptor): PluginUiPage {
  const children: PluginUiNode[] = [];
  for (const section of page.sections) {
    children.push(...adaptSection(section));
  }
  // A page body is always a stack; the legacy sections become its children
  // in order, preserving the visual sequence exactly.
  const adapted: PluginUiPage = {
    id: page.id,
    pluginId: page.pluginId,
    title: page.title,
    body: { type: 'stack', children },
  };
  if (page.icon) adapted.icon = page.icon;
  return adapted;
}
