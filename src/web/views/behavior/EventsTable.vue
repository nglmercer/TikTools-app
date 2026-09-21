<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import { IconPencil, IconTrash } from '../../components/icons.vue';
import { Icon } from '../../components/icons/index.ts';
import { Switch } from '../../components/ui/Checkbox.vue';
import { SearchInput } from '../../components/ui/TextInput.vue';
import { Tooltip } from '../../components/ui/Tooltip.vue';
import {
  createEvent,
  describeFilter,
  sortBehaviorRows,
  SortControl,
  SortHeader,
  triggerLabel,
  type SortMode,
} from './helpers.vue';
import type {
  LiveAction,
  LiveEvent,
  PluginEventType,
} from '../../../automation/behavior/types.ts';
import { t, type Locale } from '../../i18n.ts';
import { useDialogs } from '../../composables/useDialogs.ts';

type EventsTableProps = {
  locale: Locale;
  events: LiveEvent[];
  actions: LiveAction[];
  eventTypes: PluginEventType[];
  onSetEnabled: (id: string, enabled: boolean) => void;
  onDelete: (id: string) => void;
  onEdit: (event: LiveEvent) => void;
  onNew: (event: LiveEvent) => void;
};

/** Searchable, sortable events section: header tools, rows table, and empty state. */
export const EventsTable = defineVueComponent<EventsTableProps>(
  ['locale', 'events', 'actions', 'eventTypes', 'onSetEnabled', 'onDelete', 'onEdit', 'onNew'],
  (props) => {
  const query = ref('');
  const sort = ref<SortMode>('name');
  const dialogs = useDialogs();

  return () => {
  const locale = props.locale;
  const visibleEvents = props.events.filter((event) =>
    !query.value.trim()
    || event.name.toLowerCase().includes(query.value.trim().toLowerCase())
    || event.trigger.includes(query.value.trim().toLowerCase()));
  const sortedEvents = sortBehaviorRows(visibleEvents, sort.value);

  return (
    <div class="plg-section">
      <div class="plg-section__head">
        <div class="plg-section__title">
          <Tooltip text={t(locale, 'behavior.copy.events')} position="right">
            <span class="plg-section__icon" aria-hidden="true">
              <Icon name="radio" size={16} />
            </span>
          </Tooltip>
          <h3>{t(locale, 'behavior.copy.events')}</h3>
          <span class="plg-section__count">{props.events.length}</span>
        </div>
        <div class="plg-section__tools">
          <SearchInput
            name="eventQuery"
            value={query.value}
            onValueChange={(next) => { query.value = next; }}
            placeholder={t(locale, 'behavior.copy.searchEvent')}
          />
          <span class="plg-section__sort">
            <SortControl locale={locale} value={sort.value} onChange={(value) => { sort.value = value; }} />
          </span>
          <Tooltip text={t(locale, 'behavior.copy.newEvent')} position="left">
            <button
              type="button"
              class="plg-btn plg-btn--primary plg-btn--sm plg-section__new"
              onClick={() => props.onNew(createEvent(locale))}
            >
              <Icon name="plus" size={14} />
              <span>{t(locale, 'behavior.copy.newEvent')}</span>
            </button>
          </Tooltip>
        </div>
      </div>

      <div class="plg-table plg-table--events">
        <div class="plg-table__head">
          <SortHeader label={t(locale, 'behavior.copy.colActive')} sort={sort.value} onSort={(value) => { sort.value = value; }} by="enabled" />
          <SortHeader label={t(locale, 'behavior.copy.colName')} sort={sort.value} onSort={(value) => { sort.value = value; }} by="name" />
          <span>{t(locale, 'behavior.copy.colTrigger')}</span>
          <span>{t(locale, 'behavior.copy.colFilters')}</span>
          <span>{t(locale, 'behavior.copy.colActions')}</span>
          <span />
        </div>

        {sortedEvents.map((event) => (
          <div class={`plg-table__row${event.enabled ? '' : ' is-off'}`} key={event.id}>
            <Switch
              checked={event.enabled}
              onCheckedChange={() => props.onSetEnabled(event.id, !event.enabled)}
              ariaLabel={event.name}
            />
            <button
              type="button"
              class="plg-table__link"
              onClick={() => props.onEdit(event)}
            >
              {event.name}
            </button>
            <span class="plg-table__origin">{triggerLabel(event.trigger, props.eventTypes, locale)}</span>
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
                  {props.actions.find((action) => action.id === id)?.name ?? id}
                </span>
              ))}
            </span>
            <span class="plg-table__actions">
              <Tooltip text={t(locale, 'behavior.copy.edit')} position="left">
                <button
                  type="button"
                  class="plg-iconbtn"
                  aria-label={t(locale, 'behavior.copy.edit')}
                  onClick={() => props.onEdit(event)}
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
                    if (confirmed) props.onDelete(event.id);
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
              onClick={() => props.onNew(createEvent(locale))}
            >
              <Icon name="plus" size={14} />
              <span>{t(locale, 'behavior.copy.newEvent')}</span>
            </button>
          </div>
        )}
      </div>
    </div>
  );
  };
  },
);

export default EventsTable;
</script>
