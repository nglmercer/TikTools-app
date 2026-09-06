<script lang="tsx">
import { ref, watch } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { ConditionTable } from '../../components/ui/ConditionTable.vue';
import { Switch } from '../../components/ui/Checkbox.vue';
import { Select } from '../../components/ui/Select.vue';
import { TextInput } from '../../components/ui/TextInput.vue';
import { InfoTip } from '../../components/ui/InfoTip.vue';
import { COOLDOWN_CHOICES, describeFilter, sentenceFor, triggerLabel, triggerSelectOptions } from './helpers.vue';
import type {
  BehaviorRun,
  LiveAction,
  LiveEvent,
  PluginEventType,
} from '../../../automation/behavior/types.ts';
import type { GiftCatalogEntry, ViewerRecord } from '../../../shared/messages.ts';
import { t, type Locale } from '../../i18n.ts';
import { useDialogs } from '../../composables/useDialogs.ts';

type EventEditorProps = {
  locale: Locale;
  event: LiveEvent;
  isNew: boolean;
  actions: LiveAction[];
  gifts: GiftCatalogEntry[];
  viewers: ViewerRecord[];
  error?: string;
  testRuns: BehaviorRun[];
  eventTypes?: PluginEventType[];
  onCancel: () => void;
  onSave: (event: LiveEvent) => void;
  onDelete: (id: string) => void;
  onTest: (event: LiveEvent) => void;
};

export const EventEditor = defineVueComponent<EventEditorProps>(
  ['locale', 'event', 'isNew', 'actions', 'gifts', 'viewers', 'error', 'testRuns', 'eventTypes', 'onCancel', 'onSave', 'onDelete', 'onTest'],
  (props) => {
  const draft = ref<LiveEvent>(props.event);
  const step = ref(1);
  const dialogs = useDialogs();
  watch(() => props.event, (event) => { draft.value = event; });

  const update = (patch: Partial<LiveEvent>): void => { draft.value = { ...draft.value, ...patch }; };

  return () => {
  const draftValue = draft.value;
  const stepValue = step.value;
  const chosenNames = draftValue.actionIds
    .map((id) => props.actions.find((action) => action.id === id)?.name)
    .filter((name): name is string => Boolean(name));

  const steps = [
    { number: 1, label: t(props.locale, 'behavior.copy.stepWhen'), sub: triggerLabel(draftValue.trigger, props.eventTypes ?? [], props.locale) },
    {
      number: 2,
      label: t(props.locale, 'behavior.copy.stepFilters'),
      sub: draftValue.filters.length === 0
        ? t(props.locale, 'behavior.copy.alwaysShort')
        : draftValue.filters.map((filter) => describeFilter(filter, props.locale, draftValue.trigger)).join(' · '),
    },
    { number: 3, label: t(props.locale, 'behavior.copy.stepDo'), sub: chosenNames.length === 0 ? t(props.locale, 'behavior.copy.noneYet') : chosenNames.join(' · ') },
  ];

  return (
    <div class="plg">
      <div class="plg-topbar">
        <button type="button" class="plg-btn plg-btn--icon" onClick={props.onCancel} aria-label={t(props.locale, 'behavior.copy.back')}>‹</button>
        <div class="plg-topbar__text">
          <h2 class="plg-topbar__title">{draftValue.name || t(props.locale, 'behavior.copy.newEvent')}</h2>
          <span class="plg-topbar__subtitle plg-mono">{t(props.locale, 'behavior.copy.stepOf', { step: stepValue })} · {draftValue.trigger}</span>
        </div>
        <div class="plg-topbar__actions">
          {!props.isNew && (
            <button
              type="button"
              class="plg-btn plg-btn--danger plg-btn--sm"
              onClick={async () => {
                const confirmed = await dialogs.confirm(t(props.locale, 'behavior.copy.confirmDeleteEvent'), {
                  title: t(props.locale, 'behavior.copy.remove'),
                  confirmLabel: t(props.locale, 'behavior.copy.remove'),
                  cancelLabel: t(props.locale, 'cancel'),
                  danger: true,
                });
                if (confirmed) props.onDelete(draftValue.id);
              }}
            >
              {t(props.locale, 'behavior.copy.remove')}
            </button>
          )}
          <button type="button" class="plg-btn plg-btn--primary plg-btn--sm" onClick={() => props.onSave(draftValue)}>
            {t(props.locale, 'behavior.copy.save')}
          </button>
        </div>
      </div>

      <div class="plg-scroll">
        <div class="plg-form">
          <div class="plg-form__main">
            {props.error && <div class="plg-alert">{props.error}</div>}

            <p class="plg-sentence">{sentenceFor(draftValue, props.actions, props.locale, props.eventTypes ?? [])}</p>

            <div class="plg-steps">
              {steps.map((entry) => (
                <button
                  type="button"
                  key={entry.number}
                  class={`plg-steps__item${stepValue === entry.number ? ' is-active' : ''}${stepValue > entry.number ? ' is-done' : ''}`}
                  onClick={() => { step.value = entry.number; }}
                >
                  <span class="plg-step__number">{entry.number}</span>
                  <span class="plg-steps__text">
                    <span class="plg-steps__label">{entry.label}</span>
                    <span class="plg-steps__sub">{entry.sub}</span>
                  </span>
                </button>
              ))}
            </div>

            {stepValue === 1 && (
              <div class="plg-step__body">
                <div class="plg-inline">
                  <div class="plg-field">
                    <label class="plg-label" for="eventTrigger">{t(props.locale, 'behavior.copy.trigger')}</label>
                    <Select
                      id="eventTrigger"
                      name="trigger"
                      value={draftValue.trigger}
                      options={triggerSelectOptions(props.locale, props.eventTypes ?? [])}
                      onValueChange={(next) => update({ trigger: next as LiveEvent['trigger'] })}
                    />
                  </div>
                  <div class="plg-field">
                    <label class="plg-label" for="eventName">{t(props.locale, 'behavior.copy.name')}</label>
                    <TextInput
                      id="eventName"
                      name="eventName"
                      value={draftValue.name}
                      onValueChange={(next) => update({ name: next })}
                    />
                  </div>
                </div>
              </div>
            )}

            {stepValue === 2 && (
              <div class="plg-step__body">
                <div class="plg-label-row">
                  <span class="plg-label">{t(props.locale, 'behavior.copy.stepFiltersHint')}</span>
                  <InfoTip text={t(props.locale, 'behavior.copy.orHint')} position="right" />
                </div>

                <ConditionTable
                  locale={props.locale}
                  trigger={draftValue.trigger}
                  filters={draftValue.filters}
                  gifts={props.gifts}
                  viewers={props.viewers}
                  onChange={(filters) => update({ filters })}
                />
              </div>
            )}

            {stepValue === 3 && (
              <div class="plg-step__body">
                <span class="plg-label">{t(props.locale, 'behavior.copy.pickActions')}</span>
                <div class="plg-chips" style="flex-wrap: wrap;">
                  {props.actions.map((action) => {
                    const active = draftValue.actionIds.includes(action.id);
                    return (
                      <button
                        type="button"
                        key={action.id}
                        class={`plg-chip${active ? ' is-active' : ''}`}
                        onClick={() => update({
                            actionIds: active
                            ? draftValue.actionIds.filter((id) => id !== action.id)
                            : [...draftValue.actionIds, action.id],
                        })}
                      >
                        {action.name}
                      </button>
                    );
                  })}
                  {props.actions.length === 0 && <span class="plg-note">{t(props.locale, 'behavior.copy.noActionsYet')}</span>}
                </div>

                {draftValue.actionIds.length > 1 && (
                  <div class="plg-switch-row">
                    <Switch
                      checked={draftValue.runMode === 'random'}
                      onCheckedChange={(next) => update({ runMode: next ? 'random' : 'all' })}
                      label={t(props.locale, 'behavior.copy.runMode')}
                    />
                  </div>
                )}

                <div class="plg-inline">
                  <div class="plg-field">
                    <label class="plg-label" for="eventCooldown">{t(props.locale, 'behavior.copy.cooldown')}</label>
                    <Select
                      id="eventCooldown"
                      name="cooldownMs"
                      value={String(draftValue.cooldownMs)}
                      options={COOLDOWN_CHOICES.map((ms) => ({ value: String(ms), label: ms === 0 ? t(props.locale, 'behavior.copy.noCooldown') : `${ms / 1000} s` }))}
                      onValueChange={(next) => {
                        const parsed = Number(next);
                        if (Number.isFinite(parsed)) update({ cooldownMs: parsed });
                      }}
                    />
                  </div>
                  {draftValue.cooldownMs > 0 && (
                  <div class="plg-field">
                    <div class="plg-label-row">
                      <label class="plg-label" for="eventCooldownScope">{t(props.locale, 'behavior.copy.cooldownScope')}</label>
                      <InfoTip
                        text={props.locale === 'es'
                          ? 'Por usuario: la espera cuenta para cada espectador. Global: una sola espera para todos.'
                          : 'Per viewer: the cooldown counts per person. Global: one cooldown for everyone.'}
                        position="left"
                      />
                    </div>
                    <Select
                      id="eventCooldownScope"
                      name="cooldownScope"
                      value={draftValue.cooldownScope}
                      options={[
                        { value: 'user', label: t(props.locale, 'behavior.copy.perUser') },
                        { value: 'global', label: t(props.locale, 'behavior.copy.global') },
                      ]}
                      onValueChange={(next) => update({ cooldownScope: next === 'global' ? 'global' : 'user' })}
                    />
                  </div>
                  )}
                </div>
              </div>
            )}

            <div class="plg-nav">
              <button
                type="button"
                class="plg-btn plg-btn--sm"
                disabled={stepValue === 1}
                onClick={() => { step.value = Math.max(1, step.value - 1); }}
              >
                {t(props.locale, 'behavior.copy.previous')}
              </button>
              <span class="plg-nav__spacer" />
              {stepValue < 3 ? (
                <button
                  type="button"
                  class="plg-btn plg-btn--primary plg-btn--sm"
                  onClick={() => { step.value = Math.min(3, step.value + 1); }}
                >
                  {t(props.locale, 'behavior.copy.next')}
                </button>
              ) : (
                <button type="button" class="plg-btn plg-btn--primary plg-btn--sm" onClick={() => props.onSave(draftValue)}>
                  {t(props.locale, 'behavior.copy.finish')}
                </button>
              )}
            </div>
          </div>

          <div class="plg-side">
            <button type="button" class="plg-btn plg-btn--block" onClick={() => props.onTest(draftValue)}>
              {t(props.locale, 'behavior.copy.test')}
            </button>
            {props.testRuns.map((run) => (
              <div class={`plg-panel ${run.status === 'error' ? 'plg-panel--err' : 'plg-panel--ok'}`} key={run.id}>
                <span class="plg-row__name">{run.actionName}</span>
                <span class="plg-mono">{run.error ?? run.summary}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
  };
  },
);

export default EventEditor;
</script>
