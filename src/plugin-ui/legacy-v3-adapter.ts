/**
 * Backward-compatible adapter: legacy schema-v3 pages → generic UI contract.
 *
 * Existing plugin manifests (including SonicBoom) declare pages with the
 * closed v3 section kinds (`text`, `form`, `connection`, `list`, `tts`).
 * This adapter translates each section into generic nodes so the renderer
 * only speaks the declarative contract:
 *
 * - `text` → `text` node.
 * - `form` → `form` node (same schema/uiHints fallback semantics).
 * - `connection` → `connection` node.
 * - `list` → `list` node + trailing refresh `button` node.
 * - `tts` → `tts-settings` node referencing a generated `TtsContribution`.
 *
 * The adapter is the ONLY place that walks legacy UI sections to derive
 * TTS contributions. Long-term, TTS activation keys off explicit
 * contributions (future `plugins.ui.describe`), not UI sections.
 */

import type {
  PluginPageDescriptor,
  PluginPageSection,
} from '../automation/behavior/types.ts';
import { normalizeOptionsFrom } from '../automation/plugins/declarative.ts';
import type { PluginUiNode, PluginUiPage, TtsContribution } from './contracts.ts';

export interface AdaptedLegacyPage {
  page: PluginUiPage;
  /** One contribution per `tts` section (ids `main`, `main-2`, …). */
  tts: TtsContribution[];
}

/** Derives TTS contributions from legacy v3 pages (adapter-owned walk). */
export function legacyTtsContributions(pages: readonly PluginPageDescriptor[]): TtsContribution[] {
  const contributions: TtsContribution[] = [];
  for (const page of pages) {
    let index = 0;
    for (const section of page.sections) {
      if (section.kind !== 'tts' || !section.actionType || !section.voicesFrom) continue;
      const voicesFrom = normalizeOptionsFrom(section.voicesFrom);
      if (!voicesFrom) continue;
      index += 1;
      const contribution: TtsContribution = {
        id: index === 1 ? 'main' : `main-${index}`,
        pluginId: page.pluginId,
        actionType: section.actionType,
        voicesFrom,
      };
      if (section.outputsFrom) {
        const outputsFrom = normalizeOptionsFrom(section.outputsFrom);
        if (outputsFrom) contribution.outputsFrom = outputsFrom;
      }
      contributions.push(contribution);
    }
  }
  return contributions;
}

function adaptSection(
  section: PluginPageSection,
  pushing: { tts: TtsContribution[]; ttsCount: number },
  page: PluginPageDescriptor,
): PluginUiNode[] {
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
      if (!section.actionType || !section.voicesFrom) return [];
      const voicesFrom = normalizeOptionsFrom(section.voicesFrom);
      if (!voicesFrom) return [];
      pushing.ttsCount += 1;
      const id = pushing.ttsCount === 1 ? 'main' : `main-${pushing.ttsCount}`;
      const contribution: TtsContribution = {
        id,
        pluginId: page.pluginId,
        actionType: section.actionType,
        voicesFrom,
      };
      if (section.outputsFrom) {
        const outputsFrom = normalizeOptionsFrom(section.outputsFrom);
        if (outputsFrom) contribution.outputsFrom = outputsFrom;
      }
      pushing.tts.push(contribution);
      const node: PluginUiNode = { type: 'tts-settings', contribution: id };
      if (section.title) node.title = section.title;
      return [node];
    }
  }
}

/** Adapts one legacy v3 page descriptor to the generic UI contract. */
export function adaptLegacyPage(page: PluginPageDescriptor): AdaptedLegacyPage {
  const pushing = { tts: [] as TtsContribution[], ttsCount: 0 };
  const children: PluginUiNode[] = [];
  for (const section of page.sections) {
    children.push(...adaptSection(section, pushing, page));
  }
  // A page body is always a stack; the legacy sections become its children
  // in order, preserving the current visual sequence exactly.
  const adapted: PluginUiPage = {
    id: page.id,
    pluginId: page.pluginId,
    title: page.title,
    body: { type: 'stack', children },
  };
  if (page.icon) adapted.icon = page.icon;
  return { page: adapted, tts: pushing.tts };
}
