import type { IconName } from '../../icons/icon-registry.ts';
import type { AutocompleteItem } from '../autocomplete.ts';
import { groupForTemplatePath } from './rows.ts';

/* ------------------------------------------------------------------ */
/* Semantic icons (S12): IconName only, never arbitrary SVG.            */
/* ------------------------------------------------------------------ */

/**
 * Semantic icon for one row. Presets always use the server icon; template
 * paths map by section; option rows fall back to their kind. The return
 * is always a whitelisted `IconName` — untrusted `icon` strings from
 * JSON/plugins are ignored (validated at render via `readIconName`).
 */
export function iconForSuggestion(item: Pick<AutocompleteItem, 'value' | 'kind' | 'detail'>): IconName {
  if (item.detail === 'preset' || item.kind === 'snippet') return 'globe';
  const group = groupForTemplatePath(item.value);
  if (group === 'User') return 'users';
  if (group === 'Text Intelligence') return 'sparkles';
  switch (item.kind) {
    case 'number': return 'stats';
    case 'boolean': return 'check';
    case 'object':
    case 'array': return 'json';
    case 'path': return 'link';
    default: break;
  }
  if (item.value.startsWith('event.data.')) return 'chat';
  if (item.value.startsWith('event.')) return 'code';
  if (/^https?:\/\//i.test(item.value)) return 'globe';
  return 'dot';
}
