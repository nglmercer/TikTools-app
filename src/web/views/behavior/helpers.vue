<script lang="tsx">
import { IconSelect } from '../../components/ui/IconSelect.vue';
import { SvgIcon } from '../../components/icons/index.ts';
import { OPERATOR_LABELS } from '../../components/condition-icons.vue';
import { findField } from '../../../automation/behavior/fields.ts';
import { defaultActionConfig } from '../../../automation/behavior/action-config.ts';
import {
  BEHAVIOR_TRIGGERS,
  createActionId,
  createEventId,
  readString,
} from '../../../automation/behavior/schema.ts';
import type {
  ActionTypeDefinition,
  EventFilter,
  I18nText,
  LiveAction,
  LiveEvent,
  PluginEventType,
  PluginStatus,
} from '../../../automation/behavior/types.ts';
import type { AutomationEventType, JsonObject, JsonValue } from '../../../automation/types.ts';
import { i18nText, t, type Locale } from '../../i18n.ts';

export type SortMode = 'name' | 'name-desc' | 'enabled' | 'disabled';

/** Fields whose select options come from the host on demand (`optionsFrom` in uiHints). */
export function fieldsWithOptions(uiHints?: JsonObject): Array<{ key: string; source: string }> {
  if (!uiHints || typeof uiHints !== 'object' || Array.isArray(uiHints)) return [];
  const fields = uiHints.fields;
  if (!fields || typeof fields !== 'object' || Array.isArray(fields)) return [];
  const result: Array<{ key: string; source: string }> = [];
  for (const [key, hint] of Object.entries(fields as JsonObject)) {
    if (hint && typeof hint === 'object' && !Array.isArray(hint)) {
      const source = (hint as JsonObject).optionsFrom;
      if (typeof source === 'string' && /^[a-z][a-z0-9._-]{0,63}$/.test(source)) result.push({ key, source });
    }
  }
  return result;
}

export const TRIGGER_LABELS: Record<AutomationEventType, I18nText> = {
  "tiktok.chat": { default: "Someone comments", i18key: "behavior.trigger.tiktok.chat" },
  "tiktok.gift": { default: "Someone sends a gift", i18key: "behavior.trigger.tiktok.gift" },
  "tiktok.like": { default: "Someone likes", i18key: "behavior.trigger.tiktok.like" },
  "tiktok.follow": { default: "Someone follows", i18key: "behavior.trigger.tiktok.follow" },
  "tiktok.share": { default: "Someone shares", i18key: "behavior.trigger.tiktok.share" },
  "tiktok.join": { default: "Someone joins the live", i18key: "behavior.trigger.tiktok.join" },
  "tiktok.social": { default: "Social action", i18key: "behavior.trigger.tiktok.social" },
  "tiktok.room_stats": { default: "Room stats", i18key: "behavior.trigger.tiktok.room_stats" },
  "tiktok.connected": { default: "Connected", i18key: "behavior.trigger.tiktok.connected" },
  "tiktok.disconnected": { default: "Disconnected", i18key: "behavior.trigger.tiktok.disconnected" },
  "points.awarded": { default: "Points awarded", i18key: "behavior.trigger.points.awarded" },
  "plugin.emit": { default: "Internal event", i18key: "behavior.trigger.plugin.emit" },
};

export const COOLDOWN_CHOICES = [0, 3_000, 5_000, 10_000, 30_000, 60_000];

/** Flat trigger options for the event picker: built-ins first, then plugin types. */
export function triggerSelectOptions(locale: Locale, eventTypes: PluginEventType[]): Array<{ value: string; label: string }> {
  const builtin = BEHAVIOR_TRIGGERS.map((trigger) => ({ value: trigger, label: i18nText(locale, TRIGGER_LABELS[trigger]) }));
  const plugin = eventTypes.map((entry) => ({
    value: entry.type,
    label: i18nText(locale, entry.title) + ' (' + (entry.source.kind === 'plugin' ? entry.source.pluginId : 'plugin') + ')',
  }));
  return [...builtin, ...plugin];
}

/** Display label for any trigger: built-in, plugin-declared, or raw fallback. */
export function triggerLabel(trigger: string, eventTypes: PluginEventType[], locale: Locale): string {
  const builtin = (TRIGGER_LABELS as Partial<Record<string, I18nText>>)[trigger];
  if (builtin) return i18nText(locale, builtin);
  const declared = eventTypes.find((entry) => entry.type === trigger);
  if (declared) return i18nText(locale, declared.title);
  return trigger;
}

/** Helpers to slice a form schema down to the fields a tab owns. */
export function objectPropertiesOf(value: JsonValue | undefined): Record<string, JsonObject> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  return Object.fromEntries(
    Object.entries(value).filter((entry): entry is [string, JsonObject] => Boolean(entry[1]) && typeof entry[1] === 'object' && !Array.isArray(entry[1])),
  );
}

export function pickForm(form: { schema: JsonObject; uiHints?: JsonObject }, keys: string[]): { schema: JsonObject; uiHints?: JsonObject } {
  const properties = objectPropertiesOf(form.schema.properties);
  const pickedProperties: JsonObject = {};
  for (const key of keys) if (properties[key]) pickedProperties[key] = properties[key] as JsonValue;
  const fields = objectPropertiesOf(
    form.uiHints && typeof form.uiHints.fields === 'object' && !Array.isArray(form.uiHints.fields)
      ? form.uiHints.fields as JsonObject
      : undefined,
  );
  const pickedFields: JsonObject = {};
  for (const key of keys) if (fields[key]) pickedFields[key] = fields[key] as JsonValue;
  return {
    schema: { ...form.schema, properties: pickedProperties },
    uiHints: form.uiHints ? { ...form.uiHints, fields: pickedFields } : undefined,
  };
}

export function stripAdvanced(form: { schema: JsonObject; uiHints?: JsonObject }): { schema: JsonObject; uiHints?: JsonObject } {
  if (!form.uiHints || typeof form.uiHints.fields !== 'object' || Array.isArray(form.uiHints.fields)) return form;
  const fields: JsonObject = {};
  for (const [key, hint] of Object.entries(form.uiHints.fields as JsonObject)) {
    if (hint && typeof hint === 'object' && !Array.isArray(hint)) {
      const { advanced: _dropped, ...rest } = hint as JsonObject;
      void _dropped;
      fields[key] = rest as JsonValue;
    } else {
      fields[key] = hint as JsonValue;
    }
  }
  return { schema: form.schema, uiHints: { ...form.uiHints, fields } };
}

export function fieldTitle(schema: JsonObject | undefined, locale: Locale): string {
  if (!schema) return '';
  const title = (schema as JsonObject).title;
  return i18nText(locale, title);
}

export function fieldHint(hint: JsonObject | undefined, locale: Locale): string {
  if (!hint) return '';
  return i18nText(locale, (hint as JsonObject).hint);
}

export function fieldPlaceholder(hint: JsonObject | undefined): string | undefined {
  if (!hint) return undefined;
  const placeholder = (hint as JsonObject).placeholder;
  return typeof placeholder === 'string' ? placeholder : undefined;
}

export function methodOptions(
  schema: JsonObject | undefined,
  hint: JsonObject | undefined,
  locale: Locale,
): Array<{ value: string; label: string }> {
  const values = schema && Array.isArray((schema as JsonObject).enum)
    ? ((schema as JsonObject).enum as JsonValue[]).filter((entry): entry is string => typeof entry === 'string')
    : [];
  const hinted = hint && Array.isArray((hint as JsonObject).options)
    ? ((hint as JsonObject).options as JsonValue[]).filter((entry): entry is JsonObject => Boolean(entry) && typeof entry === 'object' && !Array.isArray(entry))
    : [];
  const fallback = values.length > 0 ? values : ['GET', 'POST', 'PUT', 'DELETE'];
  return fallback.map((value) => {
    const labeled = hinted.find((entry) => entry.value === value);
    return { value, label: labeled ? i18nText(locale, labeled.label) || value : value };
  });
}

/** A clickable column header that toggles between its two directions. */
export function SortHeader({
  label,
  sort,
  onSort,
  by,
}: {
  label: string;
  sort: SortMode;
  onSort: (sort: SortMode) => void;
  by: 'name' | 'enabled';
}) {
  const modes: SortMode[] = by === 'name' ? ['name', 'name-desc'] : ['enabled', 'disabled'];
  const index = modes.indexOf(sort);
  const active = index >= 0;

  return (
    <button
      type="button"
      class={`plg-sorth${active ? ' is-active' : ''}`}
      aria-label={label}
      onClick={() => onSort(modes[index === 0 ? 1 : 0]!)}
    >
      {label}
      <SvgIcon size={11} strokeWidth={2.4}>
        {active && index === 1 ? <path d="m6 9 6 6 6-6" /> : <path d="m6 15 6-6 6 6" />}
      </SvgIcon>
    </button>
  );
}

/** The same four orders as the headers, for the card layout on narrow screens. */
export function SortControl({
  locale,
  value,
  onChange,
}: {
  locale: Locale;
  value: SortMode;
  onChange: (sort: SortMode) => void;
}) {
  return (
    <IconSelect
      class="plg-sort"
      ariaLabel={t(locale, 'behavior.copy.sortBy')}
      value={value}
      onChange={(next) => onChange(next as SortMode)}
      options={[
        { value: 'name', label: t(locale, 'behavior.copy.sortName'), icon: <SortGlyph direction="up" /> },
        { value: 'name-desc', label: t(locale, 'behavior.copy.sortNameDesc'), icon: <SortGlyph direction="down" /> },
        { value: 'enabled', label: t(locale, 'behavior.copy.sortActive'), icon: <SortGlyph direction="dot" /> },
        { value: 'disabled', label: t(locale, 'behavior.copy.sortInactive'), icon: <SortGlyph direction="dot-off" /> },
      ]}
    />
  );
}

export function SortGlyph({ direction }: { direction: 'up' | 'down' | 'dot' | 'dot-off' }) {
  return (
    <SvgIcon size={13}>
      {direction === 'up' && <path d="M6 16V5m0 0L3 8m3-3 3 3M12 6h9M12 12h6M12 18h3" />}
      {direction === 'down' && <path d="M6 5v11m0 0 3-3m-3 3-3-3M12 6h9M12 12h6M12 18h3" />}
      {direction === 'dot' && <path d="M5 8h14M5 16h14" />}
      {direction === 'dot-off' && <path d="M5 8h14M5 16h14M4 4l16 16" />}
    </SvgIcon>
  );
}

export function availableActionTypes(plugins: PluginStatus[], actionTypes: ActionTypeDefinition[]): Set<string> {
  const ids = new Set<string>();
  for (const type of actionTypes) if (type.source.kind === 'builtin') ids.add(type.id);
  for (const plugin of plugins) {
    if (!plugin.installed || !plugin.enabled) continue;
    for (const type of actionTypes) {
      if (type.source.kind === 'plugin' && type.source.pluginId === plugin.descriptor.id) ids.add(type.id);
    }
  }
  return ids;
}

export function createActionFromType(type: ActionTypeDefinition, locale: Locale): LiveAction {
  const config = defaultActionConfig(type);
  return {
    schemaVersion: 2,
    id: createActionId(),
    name: i18nText(locale, type.title),
    typeId: type.id,
    enabled: true,
    config,
  };
}

export function createEvent(locale: Locale): LiveEvent {
  return {
    schemaVersion: 1,
    id: createEventId(),
    name: t(locale, 'newEventDefault'),
    enabled: false,
    trigger: 'tiktok.gift',
    filters: [],
    cooldownMs: 0,
    cooldownScope: 'user',
    actionIds: [],
    runMode: 'all',
  };
}

export function originLabel(type: ActionTypeDefinition, _locale: Locale, builtInLabel: string): string {
  const source = type.source;
  if (source.kind === 'builtin') return builtInLabel;
  return source.pluginId;
}

export function describeAction(action: LiveAction): string {
  switch (action.typeId) {
    case 'core.fetch': {
      const url = readString(action.config.url).replace(/^https?:\/\//, '').split('/')[0] ?? '';
      return `${readString(action.config.method)} ${url}`;
    }
    case 'core.emit':
      return `emit ${readString(action.config.type)}`;
    case 'core.points':
      return `${readString(action.config.delta)} · ${readString(action.config.uniqueId)}`;
    case 'core.delay':
      return `${readString(action.config.ms)} ms`;
    case 'core.log':
      return readString(action.config.message);
    case 'audio.play':
      return readString(action.config.file);
    default:
      return action.typeId;
  }
}

export function describeFilter(filter: EventFilter, locale: Locale, trigger?: string): string {
  const operator = i18nText(locale, OPERATOR_LABELS[filter.operator]);
  const field = (trigger && findField(trigger, filter.path) && i18nText(locale, findField(trigger, filter.path)!.label))
    ?? filter.path.replace(/^event\.(data|user)\./, '');
  if (filter.operator === 'is-true' || filter.operator === 'is-false') return `${field} ${operator}`;
  // A filter with no value yet reads as an ellipsis instead of a dangling word.
  const value = filter.operator === 'in' ? (filter.values ?? []).join(', ') : filter.value;
  return `${field} ${operator} ${value || '…'}`;
}

export function sentenceFor(event: LiveEvent, actions: LiveAction[], locale: Locale, eventTypes: PluginEventType[] = []): string {
  const trigger = triggerLabel(event.trigger, eventTypes, locale).toLowerCase();
  const filters = event.filters.map((filter) => describeFilter(filter, locale, event.trigger));
  const names = event.actionIds
    .map((id) => actions.find((action) => action.id === id)?.name)
    .filter((name): name is string => Boolean(name));

  const join = (items: string[]): string => {
    if (items.length === 0) return '—';
    if (items.length === 1) return items[0] as string;
    const last = items[items.length - 1] as string;
    return `${items.slice(0, -1).join(', ')} ${t(locale, 'andWord')} ${last}`;
  };

  if (locale === 'es') {
    const condition = filters.length ? ` y ${join(filters)}` : '';
    const what = event.runMode === 'random' ? `una de: ${join(names)}` : join(names);
    return `Cuando ${trigger}${condition}, ejecuta ${what}.`;
  }

  const condition = filters.length ? ` and ${join(filters)}` : '';
  const what = event.runMode === 'random' ? `one of: ${join(names)}` : join(names);
  return `When ${trigger}${condition}, run ${what}.`;
}

export function relativeTime(timestamp: number, locale: Locale): string {
  const seconds = Math.max(0, Math.round((Date.now() - timestamp) / 1000));
  if (seconds < 5) return t(locale, 'nowWord');
  if (seconds < 60) return `${seconds} s`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min`;
  return `${Math.round(minutes / 60)} h`;
}

export default SortHeader;
</script>
