<script lang="tsx">
import { computed, ref } from 'vue';
import { defineVueComponent } from '../vue/component.ts';
import { IconPencil, IconTrash } from '../components/icons.vue';
import { Icon } from '../components/icons/index.ts';
import { Switch } from '../components/ui/Checkbox.vue';
import { SearchInput } from '../components/ui/TextInput.vue';
import { Tooltip } from '../components/ui/Tooltip.vue';
import { ActionEditor } from './behavior/action-editor.vue';
import { ActionPicker } from './behavior/action-picker.vue';
import { EventEditor } from './behavior/event-editor.vue';
import {
  availableActionTypes,
  createActionFromType,
  createEvent,
  describeAction,
  describeFilter,
  originLabel,
  relativeTime,
  SortControl,
  SortHeader,
  triggerLabel,
  type SortMode,
} from './behavior/helpers.vue';
import type {
  BehaviorRun,
  BehaviorSnapshot,
  LiveAction,
  LiveEvent,
} from '../../automation/behavior/types.ts';
import type { ActionOptionItem, GiftCatalogEntry, HotkeyStatusData, OpenMediaPicker, ViewerRecord } from '../../shared/messages.ts';
import { t, type Locale } from '../i18n.ts';
import { useDialogs } from '../composables/useDialogs.ts';
import { formatHotkeyChord, hotkeyListenerState, summarizeHotkeyStatus } from '../components/ui/hotkey-status.ts';
import type { LastHotkeyEvent } from '../features/automation.ts';

type BehaviorViewProps = {
  locale: Locale;
  snapshot: BehaviorSnapshot;
  /** Sources for the value pickers: the room's gifts and the known viewers. */
  gifts: GiftCatalogEntry[];
  viewers: ViewerRecord[];
  runs: BehaviorRun[];
  testRuns: BehaviorRun[];
  hotkeyStatus?: HotkeyStatusData | null;
  lastHotkeyEvent?: LastHotkeyEvent | null;
  error?: string;
  onSaveAction: (action: LiveAction) => void;
  onDeleteAction: (id: string) => void;
  onSetActionEnabled: (id: string, enabled: boolean) => void;
  onTestAction: (action: LiveAction, trigger?: string) => void;
  onSaveEvent: (event: LiveEvent) => void;
  onDeleteEvent: (id: string) => void;
  onSetEventEnabled: (id: string, enabled: boolean) => void;
  onTestEvent: (event: LiveEvent) => void;
  onOpenPlugins: () => void;
  onOpenMediaPicker: OpenMediaPicker;
  /** On-demand option lists keyed by options source. */
  actionOptions: Record<string, ActionOptionItem[]>;
  /** Per-source fetch errors for the option lists above. */
  actionOptionErrors: Record<string, string>;
  onGetActionOptions: (source: string, refresh?: boolean) => void;
};

type Screen =
  | { kind: 'list' }
  | { kind: 'picker' }
  | { kind: 'action'; action: LiveAction; isNew: boolean }
  | { kind: 'event'; event: LiveEvent; isNew: boolean };

export const BehaviorView = defineVueComponent<BehaviorViewProps>(
  [
    'locale',
    'snapshot',
    'gifts',
    'viewers',
    'runs',
    'testRuns',
    'hotkeyStatus',
    'lastHotkeyEvent',
    'error',
    'onSaveAction',
    'onDeleteAction',
    'onSetActionEnabled',
    'onTestAction',
    'onSaveEvent',
    'onDeleteEvent',
    'onSetEventEnabled',
    'onTestEvent',
    'onOpenPlugins',
    'onOpenMediaPicker',
    'actionOptions',
    'actionOptionErrors',
    'onGetActionOptions',
  ],
  (props) => {
  const screen = ref<Screen>({ kind: 'list' });
  const actionQuery = ref('');
  const eventQuery = ref('');
  const actionSort = ref<SortMode>('name');
  const eventSort = ref<SortMode>('name');
  const dialogs = useDialogs();

  const lastRunByAction = computed(() => {
    const map = new Map<string, BehaviorRun>();
    for (const run of props.runs) {
      if (run.test || !run.actionId) continue;
      if (!map.has(run.actionId)) map.set(run.actionId, run);
    }
    return map;
  });

  const availableTypes = computed(() => availableActionTypes(props.snapshot.plugins, props.snapshot.actionTypes));

  return () => {
  const locale = props.locale;
  const snapshot = props.snapshot;
  const runs = props.runs;
  const testRuns = props.testRuns;
  const error = props.error;
  const currentScreen = screen.value;
  const hotkeySummary = props.hotkeyStatus ? summarizeHotkeyStatus(props.hotkeyStatus) : null;
  // Always-visible listener status while the plugin is installed: healthy
  // and starting states must be as obvious as failures.
  const hotkeyPanel = (() => {
    const plugin = snapshot.plugins.find((entry) => entry.descriptor.id === 'hotkeys');
    if (!plugin?.installed) return null;
    if (!plugin.enabled) {
      return {
        tone: 'idle' as const,
        headline: t(locale, 'hotkeyStateDisabled'),
        lines: [] as string[],
        lastEvent: null as string | null,
      };
    }
    const state = hotkeyListenerState(props.hotkeyStatus);
    const headline = state === 'active'
      ? `${t(locale, 'hotkeyStateActive')} · ${(hotkeySummary?.lines ?? []).find((line) => line.includes('via')) ?? hotkeySummary?.headline ?? ''}`
      : state === 'starting' || state === 'unknown'
        ? t(locale, 'hotkeyStateStarting')
        : state === 'permission'
          ? t(locale, 'hotkeyStatePermission')
          : state === 'failed'
            ? t(locale, 'hotkeyStateFailed')
            : t(locale, 'hotkeyStateUnsupported');
    const tone = state === 'active' ? 'ok' as const : (state === 'permission' || state === 'failed') ? 'err' as const : 'idle' as const;
    const summary = hotkeySummary;
    const lines = summary && (summary.needsAttention || state !== 'active')
      ? summary.lines.filter((line) => line !== summary.headline)
      : [];
    const last = props.lastHotkeyEvent;
    const lastEvent = last
      ? `${t(locale, 'hotkeyLastEvent')}: ${formatHotkeyChord(last.key, last.modifiers)} · ${relativeTime(last.at, locale)}`
      : state === 'active' || state === 'starting' || state === 'unknown'
        ? t(locale, 'hotkeyStateNoEvents')
        : null;
    return { tone, headline, lines, lastEvent };
  })();

  if (currentScreen.kind === 'picker') {
    return (
      <ActionPicker
        locale={locale}
        plugins={snapshot.plugins}
        onCancel={() => { screen.value = { kind: 'list' }; }}
        onOpenPlugins={props.onOpenPlugins}
        actionTypes={snapshot.actionTypes}
        onPick={(type) => { screen.value = { kind: 'action', action: createActionFromType(type, locale), isNew: true }; }}
      />
    );
  }

  if (currentScreen.kind === 'action') {
    return (
      <ActionEditor
        key={currentScreen.action.id}
        locale={locale}
        action={currentScreen.action}
        actionTypes={snapshot.actionTypes}
        isNew={currentScreen.isNew}
        error={error}
        testRuns={testRuns}
        actionOptions={props.actionOptions}
        actionOptionErrors={props.actionOptionErrors}
        onGetActionOptions={props.onGetActionOptions}
        onOpenMediaPicker={props.onOpenMediaPicker}
        onCancel={() => { screen.value = { kind: 'list' }; }}
        onSave={(action) => {
          props.onSaveAction(action);
          screen.value = { kind: 'list' };
        }}
        onDelete={(id) => {
          props.onDeleteAction(id);
          screen.value = { kind: 'list' };
        }}
        onTest={props.onTestAction}
      />
    );
  }

  if (currentScreen.kind === 'event') {
    return (
      <EventEditor
        key={currentScreen.event.id}
        locale={locale}
        event={currentScreen.event}
        isNew={currentScreen.isNew}
        actions={snapshot.actions}
        eventTypes={snapshot.eventTypes ?? []}
        hotkeyStatus={props.hotkeyStatus}
        gifts={props.gifts}
        viewers={props.viewers}
        error={error}
        testRuns={testRuns}
        onCancel={() => { screen.value = { kind: 'list' }; }}
        onSave={(event) => {
          props.onSaveEvent(event);
          screen.value = { kind: 'list' };
        }}
        onDelete={(id) => {
          props.onDeleteEvent(id);
          screen.value = { kind: 'list' };
        }}
        onTest={props.onTestEvent}
      />
    );
  }

  const sortRows = <T extends { name: string; enabled: boolean }>(rows: T[], sort: SortMode): T[] =>
    [...rows].sort((left, right) => {
      if (sort === 'enabled' || sort === 'disabled') {
        const delta = Number(right.enabled) - Number(left.enabled);
        if (delta !== 0) return sort === 'enabled' ? delta : -delta;
        return left.name.localeCompare(right.name);
      }
      const byName = left.name.localeCompare(right.name);
      return sort === 'name-desc' ? -byName : byName;
    });

  const visibleActions = snapshot.actions.filter((action) =>
    !actionQuery.value.trim() || action.name.toLowerCase().includes(actionQuery.value.trim().toLowerCase()));
  const visibleEvents = snapshot.events.filter((event) =>
    !eventQuery.value.trim()
    || event.name.toLowerCase().includes(eventQuery.value.trim().toLowerCase())
    || event.trigger.includes(eventQuery.value.trim().toLowerCase()));
  const sortedActions = sortRows(visibleActions, actionSort.value);
  const sortedEvents = sortRows(visibleEvents, eventSort.value);

  return (
    <div class="plg">
      {error && <div class="plg-stack"><div class="plg-alert">{error}</div></div>}

      {hotkeyPanel && (
        <div class="plg-stack plg-hotkey-status" role="status" aria-live="polite">
          <div class={`plg-panel plg-hotkey-status__panel${hotkeyPanel.tone === 'err' ? ' plg-panel--err' : hotkeyPanel.tone === 'ok' ? ' plg-panel--ok' : ''}`}>
            <div class="plg-hotkey-status__head">
              <span class={`plg-dot${hotkeyPanel.tone === 'err' ? ' is-err' : hotkeyPanel.tone === 'ok' ? ' is-ok' : ''}`} />
              <strong>{t(locale, 'hotkeyStatusTitle')}</strong>
              <span class="plg-hotkey-status__headline">{hotkeyPanel.headline}</span>
            </div>
            {hotkeyPanel.lines.map((line) => <span class="plg-hotkey-status__line plg-mono" key={line}>{line}</span>)}
            {hotkeyPanel.lastEvent && <span class="plg-hotkey-status__line plg-mono">{hotkeyPanel.lastEvent}</span>}
          </div>
        </div>
      )}

      <div class="plg-body">
        <div class="plg-scroll">
          <div class="plg-section">
            <div class="plg-section__head">
              <div class="plg-section__title">
                <Tooltip text={t(locale, 'behavior.copy.actions')} position="right">
                  <span class="plg-section__icon" aria-hidden="true">
                    <Icon name="bolt" size={16} />
                  </span>
                </Tooltip>
                <h3>{t(locale, 'behavior.copy.actions')}</h3>
                <span class="plg-section__count">{snapshot.actions.length}</span>
              </div>
              <div class="plg-section__tools">
                <SearchInput
                  name="actionQuery"
                  value={actionQuery.value}
                  onValueChange={(next) => { actionQuery.value = next; }}
                  placeholder={t(locale, 'behavior.copy.searchAction')}
                />
                <span class="plg-section__sort">
                  <SortControl locale={locale} value={actionSort.value} onChange={(value) => { actionSort.value = value; }} />
                </span>
                <Tooltip text={t(locale, 'behavior.copy.newAction')} position="left">
                  <button type="button" class="plg-btn plg-btn--primary plg-btn--sm plg-section__new" onClick={() => { screen.value = { kind: 'picker' }; }}>
                    <Icon name="plus" size={14} />
                    <span>{t(locale, 'behavior.copy.newAction')}</span>
                  </button>
                </Tooltip>
              </div>
            </div>

            <div class="plg-table plg-table--actions">
              <div class="plg-table__head">
                <SortHeader
                  label={t(locale, 'behavior.copy.colActive')}
                  sort={actionSort.value}
                  onSort={(value) => { actionSort.value = value; }}
                  by="enabled"
                />
                <SortHeader label={t(locale, 'behavior.copy.colName')} sort={actionSort.value} onSort={(value) => { actionSort.value = value; }} by="name" />
                <span>{t(locale, 'behavior.copy.colOrigin')}</span>
                <span>{t(locale, 'behavior.copy.colDoes')}</span>
                <span>{t(locale, 'behavior.copy.colLast')}</span>
                <span />
              </div>

              {sortedActions.map((action) => {
                const type = snapshot.actionTypes.find((entry) => entry.id === action.typeId);
                const lastRun = lastRunByAction.value.get(action.id);
                const failing = lastRun?.status === 'error';
                const usable = !type || type.source.kind === 'builtin' || availableTypes.value.has(action.typeId);
                return (
                  <div
                    class={`plg-table__row${action.enabled ? '' : ' is-off'}${failing && action.enabled ? ' has-error' : ''}`}
                    key={action.id}
                  >
                    <Switch
                      checked={action.enabled}
                      onCheckedChange={() => props.onSetActionEnabled(action.id, !action.enabled)}
                      ariaLabel={action.name}
                    />
                    <button
                      type="button"
                      class="plg-table__link"
                      onClick={() => { screen.value = { kind: 'action', action, isNew: false }; }}
                    >
                      {action.name}
                    </button>
                    <span class="plg-table__meta">
                      <span class="plg-table__origin">
                        {type ? originLabel(type, locale, t(locale, 'behavior.copy.builtIn')) : '—'}
                        {!usable && ` · ${t(locale, 'behavior.copy.pluginMissing')}`}
                      </span>
                      <span class="plg-pill plg-pill--mono">{type?.tag ?? '—'}</span>
                    </span>
                    <span class="plg-table__detail">{describeAction(action)}</span>
                    <span class={`plg-table__status${!action.enabled ? '' : failing ? ' is-err' : lastRun ? ' is-ok' : ''}`}>
                      <span class={`plg-dot${!action.enabled ? '' : failing ? ' is-err' : lastRun ? ' is-ok' : ''}`} />
                      {!action.enabled
                        ? t(locale, 'behavior.copy.paused')
                        : lastRun
                          ? `${lastRun.error ?? lastRun.summary} · ${relativeTime(lastRun.at, locale)}`
                          : t(locale, 'behavior.copy.noRuns')}
                    </span>
                    <span class="plg-table__actions">
                      <Tooltip text={t(locale, 'behavior.copy.edit')} position="left">
                        <button
                          type="button"
                          class="plg-iconbtn"
                          aria-label={t(locale, 'behavior.copy.edit')}
                          onClick={() => { screen.value = { kind: 'action', action, isNew: false }; }}
                        >
                          <IconPencil />
                        </button>
                      </Tooltip>
                      <Tooltip text={t(locale, 'behavior.copy.remove')} position="left">
                        <button
                          type="button"
                          class="plg-iconbtn is-danger"
                          aria-label={t(locale, 'behavior.copy.remove')}
                          onClick={async () => {
                            const confirmed = await dialogs.confirm(t(locale, 'behavior.copy.confirmDeleteAction'), {
                              title: t(locale, 'behavior.copy.remove'),
                              confirmLabel: t(locale, 'behavior.copy.remove'),
                              cancelLabel: t(locale, 'cancel'),
                              danger: true,
                            });
                            if (confirmed) props.onDeleteAction(action.id);
                          }}
                        >
                          <IconTrash />
                        </button>
                      </Tooltip>
                    </span>
                  </div>
                );
              })}

              {visibleActions.length === 0 && (
                <div class="plg-empty">
                  <span class="plg-empty__desc">{t(locale, 'behavior.copy.noActions')}</span>
                  <button type="button" class="plg-btn plg-btn--primary" onClick={() => { screen.value = { kind: 'picker' }; }}>
                    <Icon name="plus" size={14} />
                    <span>{t(locale, 'behavior.copy.newAction')}</span>
                  </button>
                </div>
              )}
            </div>
          </div>

          <div class="plg-section">
            <div class="plg-section__head">
              <div class="plg-section__title">
                <Tooltip text={t(locale, 'behavior.copy.events')} position="right">
                  <span class="plg-section__icon" aria-hidden="true">
                    <Icon name="radio" size={16} />
                  </span>
                </Tooltip>
                <h3>{t(locale, 'behavior.copy.events')}</h3>
                <span class="plg-section__count">{snapshot.events.length}</span>
              </div>
              <div class="plg-section__tools">
                <SearchInput
                  name="eventQuery"
                  value={eventQuery.value}
                  onValueChange={(next) => { eventQuery.value = next; }}
                  placeholder={t(locale, 'behavior.copy.searchEvent')}
                />
                <span class="plg-section__sort">
                  <SortControl locale={locale} value={eventSort.value} onChange={(value) => { eventSort.value = value; }} />
                </span>
                <Tooltip text={t(locale, 'behavior.copy.newEvent')} position="left">
                  <button
                    type="button"
                    class="plg-btn plg-btn--primary plg-btn--sm plg-section__new"
                    onClick={() => { screen.value = { kind: 'event', event: createEvent(locale), isNew: true }; }}
                  >
                    <Icon name="plus" size={14} />
                    <span>{t(locale, 'behavior.copy.newEvent')}</span>
                  </button>
                </Tooltip>
              </div>
            </div>

            <div class="plg-table plg-table--events">
              <div class="plg-table__head">
                <SortHeader label={t(locale, 'behavior.copy.colActive')} sort={eventSort.value} onSort={(value) => { eventSort.value = value; }} by="enabled" />
                <SortHeader label={t(locale, 'behavior.copy.colName')} sort={eventSort.value} onSort={(value) => { eventSort.value = value; }} by="name" />
                <span>{t(locale, 'behavior.copy.colTrigger')}</span>
                <span>{t(locale, 'behavior.copy.colFilters')}</span>
                <span>{t(locale, 'behavior.copy.colActions')}</span>
                <span />
              </div>

              {sortedEvents.map((event) => (
                <div class={`plg-table__row${event.enabled ? '' : ' is-off'}`} key={event.id}>
                  <Switch
                    checked={event.enabled}
                    onCheckedChange={() => props.onSetEventEnabled(event.id, !event.enabled)}
                    ariaLabel={event.name}
                  />
                  <button
                    type="button"
                    class="plg-table__link"
                    onClick={() => { screen.value = { kind: 'event', event, isNew: false }; }}
                  >
                    {event.name}
                  </button>
                  <span class="plg-table__origin">{triggerLabel(event.trigger, snapshot.eventTypes ?? [], locale)}</span>
                  <span class="plg-table__chips">
                    {event.filters.length === 0 && <span class="plg-pill">{t(locale, 'behavior.copy.always')}</span>}
                    {event.filters.map((filter, index) => (
                      <span class="plg-pill plg-pill--mono" key={`${filter.path}-${index}`}>
                        {describeFilter(filter, locale, event.trigger)}
                      </span>
                    ))}
                  </span>
                  <span class="plg-table__chips">
                    {event.actionIds.map((id) => (
                      <span class="plg-pill plg-pill--accent" key={id}>
                        {snapshot.actions.find((action) => action.id === id)?.name ?? id}
                      </span>
                    ))}
                  </span>
                  <span class="plg-table__actions">
                    <Tooltip text={t(locale, 'behavior.copy.edit')} position="left">
                      <button
                        type="button"
                        class="plg-iconbtn"
                        aria-label={t(locale, 'behavior.copy.edit')}
                        onClick={() => { screen.value = { kind: 'event', event, isNew: false }; }}
                      >
                        <IconPencil />
                      </button>
                    </Tooltip>
                    <Tooltip text={t(locale, 'behavior.copy.remove')} position="left">
                      <button
                        type="button"
                        class="plg-iconbtn is-danger"
                        aria-label={t(locale, 'behavior.copy.remove')}
                        onClick={async () => {
                          const confirmed = await dialogs.confirm(t(locale, 'behavior.copy.confirmDeleteEvent'), {
                            title: t(locale, 'behavior.copy.remove'),
                            confirmLabel: t(locale, 'behavior.copy.remove'),
                            cancelLabel: t(locale, 'cancel'),
                            danger: true,
                          });
                          if (confirmed) props.onDeleteEvent(event.id);
                        }}
                      >
                        <IconTrash />
                      </button>
                    </Tooltip>
                  </span>
                </div>
              ))}

              {visibleEvents.length === 0 && (
                <div class="plg-empty">
                  <span class="plg-empty__desc">{t(locale, 'behavior.copy.noEvents')}</span>
                  <button
                    type="button"
                    class="plg-btn plg-btn--primary"
                    onClick={() => { screen.value = { kind: 'event', event: createEvent(locale), isNew: true }; }}
                  >
                    <Icon name="plus" size={14} />
                    <span>{t(locale, 'behavior.copy.newEvent')}</span>
                  </button>
                </div>
              )}
            </div>
          </div>
        </div>

        <aside class="plg-body__aside">
          <div class="plg-toolbar">
            <span class="plg-section-title">{t(locale, 'behavior.copy.runs')}</span>
          </div>
          <div class="plg-runs">
            {runs.length === 0 && <p class="plg-note">{t(locale, 'behavior.copy.runsEmpty')}</p>}
            {runs.slice(0, 20).map((run) => (
              <div class="plg-run" key={run.id}>
                <span class={`plg-dot${run.status === 'ok' ? ' is-ok' : run.status === 'error' ? ' is-err' : ''}`} />
                <div class="plg-run__text">
                  <span class="plg-run__name">{run.actionName}</span>
                  <span class="plg-run__detail">{run.error ?? run.summary}</span>
                </div>
                <span class="plg-run__time">{relativeTime(run.at, locale)}</span>
              </div>
            ))}
          </div>
        </aside>
      </div>
    </div>
  );
  };
  },
);

export default BehaviorView;
</script>
