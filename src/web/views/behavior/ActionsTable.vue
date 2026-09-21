<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import { IconPencil, IconTrash } from '../../components/icons.vue';
import { Icon } from '../../components/icons/index.ts';
import { Switch } from '../../components/ui/Checkbox.vue';
import { SearchInput } from '../../components/ui/TextInput.vue';
import { Tooltip } from '../../components/ui/Tooltip.vue';
import {
  describeAction,
  originLabel,
  relativeTime,
  sortBehaviorRows,
  SortControl,
  SortHeader,
  type SortMode,
} from './helpers.vue';
import type {
  ActionTypeDefinition,
  BehaviorRun,
  LiveAction,
} from '../../../automation/behavior/types.ts';
import { t, type Locale } from '../../i18n.ts';
import { useDialogs } from '../../composables/useDialogs.ts';

type ActionsTableProps = {
  locale: Locale;
  actions: LiveAction[];
  actionTypes: ActionTypeDefinition[];
  availableTypes: Set<string>;
  lastRunByAction: Map<string, BehaviorRun>;
  onSetEnabled: (id: string, enabled: boolean) => void;
  onDelete: (id: string) => void;
  onEdit: (action: LiveAction) => void;
  onNew: () => void;
};

/** Searchable, sortable actions section: header tools, rows table, and empty state. */
export const ActionsTable = defineVueComponent<ActionsTableProps>(
  ['locale', 'actions', 'actionTypes', 'availableTypes', 'lastRunByAction', 'onSetEnabled', 'onDelete', 'onEdit', 'onNew'],
  (props) => {
  const query = ref('');
  const sort = ref<SortMode>('name');
  const dialogs = useDialogs();

  return () => {
  const locale = props.locale;
  const visibleActions = props.actions.filter((action) =>
    !query.value.trim() || action.name.toLowerCase().includes(query.value.trim().toLowerCase()));
  const sortedActions = sortBehaviorRows(visibleActions, sort.value);

  return (
    <div class="plg-section">
      <div class="plg-section__head">
        <div class="plg-section__title">
          <Tooltip text={t(locale, 'behavior.copy.actions')} position="right">
            <span class="plg-section__icon" aria-hidden="true">
              <Icon name="bolt" size={16} />
            </span>
          </Tooltip>
          <h3>{t(locale, 'behavior.copy.actions')}</h3>
          <span class="plg-section__count">{props.actions.length}</span>
        </div>
        <div class="plg-section__tools">
          <SearchInput
            name="actionQuery"
            value={query.value}
            onValueChange={(next) => { query.value = next; }}
            placeholder={t(locale, 'behavior.copy.searchAction')}
          />
          <span class="plg-section__sort">
            <SortControl locale={locale} value={sort.value} onChange={(value) => { sort.value = value; }} />
          </span>
          <Tooltip text={t(locale, 'behavior.copy.newAction')} position="left">
            <button type="button" class="plg-btn plg-btn--primary plg-btn--sm plg-section__new" onClick={() => props.onNew()}>
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
            sort={sort.value}
            onSort={(value) => { sort.value = value; }}
            by="enabled"
          />
          <SortHeader label={t(locale, 'behavior.copy.colName')} sort={sort.value} onSort={(value) => { sort.value = value; }} by="name" />
          <span>{t(locale, 'behavior.copy.colOrigin')}</span>
          <span>{t(locale, 'behavior.copy.colDoes')}</span>
          <span>{t(locale, 'behavior.copy.colLast')}</span>
          <span />
        </div>

        {sortedActions.map((action) => {
          const type = props.actionTypes.find((entry) => entry.id === action.typeId);
          const lastRun = props.lastRunByAction.get(action.id);
          const failing = lastRun?.status === 'error';
          const usable = !type || type.source.kind === 'builtin' || props.availableTypes.has(action.typeId);
          return (
            <div
              class={`plg-table__row${action.enabled ? '' : ' is-off'}${failing && action.enabled ? ' has-error' : ''}`}
              key={action.id}
            >
              <Switch
                checked={action.enabled}
                onCheckedChange={() => props.onSetEnabled(action.id, !action.enabled)}
                ariaLabel={action.name}
              />
              <button
                type="button"
                class="plg-table__link"
                onClick={() => props.onEdit(action)}
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
                    onClick={() => props.onEdit(action)}
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
                      if (confirmed) props.onDelete(action.id);
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
            <button type="button" class="plg-btn plg-btn--primary" onClick={() => props.onNew()}>
              <Icon name="plus" size={14} />
              <span>{t(locale, 'behavior.copy.newAction')}</span>
            </button>
          </div>
        )}
      </div>
    </div>
  );
  };
  },
);

export default ActionsTable;
</script>
