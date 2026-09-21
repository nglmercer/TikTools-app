/**
 * UI action validation for the declarative plugin UI contract.
 *
 * Action descriptors are data, allowlisted by type. There is no generic
 * arbitrary-RPC action: every executable effect maps to one explicit host
 * operation with capability checks remaining host-side.
 */

import type { JsonObject, JsonValue } from '../automation/types.ts';
import { PLUGIN_UI_ACTION_TYPES, type PluginUiAction } from './contracts.ts';

const ACTION_TYPE_PATTERN = /^[a-z][a-z0-9._-]{0,127}$/;

function isRecord(value: JsonValue | undefined): value is JsonObject {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

/**
 * Validates an untrusted action descriptor. Returns the normalized action
 * or undefined when the descriptor is malformed or not allowlisted.
 * Option-source strings are length-checked here; full source validation
 * happens at execution time via the host's normalizers.
 */
export function normalizeAction(value: JsonValue | undefined): PluginUiAction | undefined {
  if (!isRecord(value) || typeof value.type !== 'string') return undefined;
  if (!(PLUGIN_UI_ACTION_TYPES as readonly string[]).includes(value.type)) return undefined;
  switch (value.type) {
    case 'save-settings':
      return { type: 'save-settings' };
    case 'plugin-action': {
      if (typeof value.actionType !== 'string' || !ACTION_TYPE_PATTERN.test(value.actionType)) {
        return undefined;
      }
      const action: PluginUiAction = { type: 'plugin-action', actionType: value.actionType };
      if (value.config !== undefined) {
        if (!isRecord(value.config)) return undefined;
        action.config = value.config;
      }
      return action;
    }
    case 'refresh-source': {
      if (typeof value.source !== 'string' || !value.source.trim() || value.source.length > 256) {
        return undefined;
      }
      return { type: 'refresh-source', source: value.source.trim() };
    }
    case 'test-connection':
      return { type: 'test-connection' };
    case 'open-media-picker': {
      const action: PluginUiAction = { type: 'open-media-picker' };
      if (value.accept !== undefined) {
        if (typeof value.accept !== 'string' || value.accept.length > 128) return undefined;
        action.accept = value.accept;
      }
      if (value.target !== undefined) {
        if (typeof value.target !== 'string' || value.target.length > 128) return undefined;
        action.target = value.target;
      }
      return action;
    }
    default:
      return undefined;
  }
}
