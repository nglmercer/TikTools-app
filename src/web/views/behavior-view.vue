<script lang="tsx">
import { computed, ref } from 'vue';
import { defineVueComponent } from '../vue/component.ts';
import { ActionEditor } from './behavior/action-editor.vue';
import { ActionPicker } from './behavior/action-picker.vue';
import { EventEditor } from './behavior/event-editor.vue';
import { ActionsTable } from './behavior/ActionsTable.vue';
import { EventsTable } from './behavior/EventsTable.vue';
import { HotkeyFloatBadge } from './behavior/HotkeyFloatBadge.vue';
import {
  availableActionTypes,
  createActionFromType,
  relativeTime,
} from './behavior/helpers.vue';
import type {
  BehaviorRun,
  BehaviorSnapshot,
  LiveAction,
  LiveEvent,
} from '../../automation/behavior/types.ts';
import type { ActionOptionItem, GiftCatalogEntry, HotkeyStatusData, OpenMediaPicker, ViewerRecord } from '../../shared/messages.ts';
import { t, type Locale } from '../i18n.ts';
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
  hotkeyAccessPending?: boolean;
  onRequestHotkeyAccess: () => void;
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
    'hotkeyAccessPending',
    'onRequestHotkeyAccess',
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

  return (
    <div class="plg plg--behavior">
      {error && <div class="plg-stack"><div class="plg-alert">{error}</div></div>}

      <HotkeyFloatBadge
        locale={locale}
        plugins={snapshot.plugins}
        hotkeyStatus={props.hotkeyStatus}
        lastHotkeyEvent={props.lastHotkeyEvent}
        accessPending={props.hotkeyAccessPending}
        onRequestAccess={props.onRequestHotkeyAccess}
      />

      <div class="plg-body">
        <div class="plg-scroll">
          <ActionsTable
            locale={locale}
            actions={snapshot.actions}
            actionTypes={snapshot.actionTypes}
            availableTypes={availableTypes.value}
            lastRunByAction={lastRunByAction.value}
            onSetEnabled={props.onSetActionEnabled}
            onDelete={props.onDeleteAction}
            onEdit={(action) => { screen.value = { kind: 'action', action, isNew: false }; }}
            onNew={() => { screen.value = { kind: 'picker' }; }}
          />
          <EventsTable
            locale={locale}
            events={snapshot.events}
            actions={snapshot.actions}
            eventTypes={snapshot.eventTypes ?? []}
            onSetEnabled={props.onSetEventEnabled}
            onDelete={props.onDeleteEvent}
            onEdit={(event) => { screen.value = { kind: 'event', event, isNew: false }; }}
            onNew={(event) => { screen.value = { kind: 'event', event, isNew: true }; }}
          />
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
