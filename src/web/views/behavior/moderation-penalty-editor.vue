<script lang="tsx">
import { computed, ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { NumberInput } from '../../components/ui/NumberInput.vue';
import { Tooltip } from '../../components/ui/Tooltip.vue';
import { IconChevronLeft } from '../../components/icons/index.ts';
import {
  MODERATION_BLOCKED_PATH,
  MODERATION_VIEWER_TEMPLATE,
  penaltyPointsError,
} from '../../../automation/behavior/moderation-penalty.ts';
import { triggerLabel } from './helpers.vue';
import { t, type Locale } from '../../i18n.ts';

type ModerationPenaltyEditorProps = {
  locale: Locale;
  error?: string;
  onCancel: () => void;
  onSave: (points: number) => void;
};

export const ModerationPenaltyEditor = defineVueComponent<ModerationPenaltyEditorProps>(
  ['locale', 'error', 'onCancel', 'onSave'],
  (props) => {
  const points = ref<number | null>(-10);
  const attempted = ref(false);

  const pointsError = computed(() => {
    const code = penaltyPointsError(points.value ?? Number.NaN);
    if (code === 'not-finite') return t(props.locale, 'behavior.copy.moderationPenaltyNotFinite');
    if (code === 'zero') return t(props.locale, 'behavior.copy.moderationPenaltyZero');
    return '';
  });

  const save = (): void => {
    attempted.value = true;
    if (pointsError.value || points.value === null) return;
    props.onSave(points.value);
  };

  return () => {
  const locale = props.locale;
  const fieldError = attempted.value ? pointsError.value : '';

  return (
    <div class="plg">
      <div class="plg-topbar">
        <Tooltip text={t(locale, 'behavior.copy.backHint')} position="bottom" wide>
          <button
            type="button"
            class="plg-btn plg-btn--icon"
            onClick={props.onCancel}
            aria-label={t(locale, 'behavior.copy.back')}
          >
            <IconChevronLeft size={16} />
          </button>
        </Tooltip>
        <div class="plg-topbar__text">
          <h2 class="plg-topbar__title">{t(locale, 'behavior.copy.moderationPenalty')}</h2>
          <span class="plg-topbar__subtitle">{t(locale, 'behavior.copy.moderationPenaltyLead')}</span>
        </div>
        <div class="plg-topbar__actions">
          <Tooltip text={t(locale, 'behavior.copy.moderationPenaltySaveHint')} position="bottom" wide>
            <button
              type="button"
              class="plg-btn plg-btn--primary plg-btn--sm"
              onClick={save}
            >
              {t(locale, 'behavior.copy.save')}
            </button>
          </Tooltip>
        </div>
      </div>

      <div class="plg-scroll">
        <div class="plg-form">
          <div class="plg-form__main">
            {props.error && <div class="plg-alert">{props.error}</div>}

            <div class="plg-field">
              <NumberInput
                label={t(locale, 'behavior.copy.moderationPenaltyPoints')}
                hint={t(locale, 'behavior.copy.moderationPenaltyPointsHint')}
                value={points.value}
                step={1}
                error={fieldError || undefined}
                onValueChange={(next) => { points.value = next; }}
              />
            </div>

            <div class="plg-field">
              <span class="act-label">{t(locale, 'behavior.copy.colTrigger')}</span>
              <p class="plg-note">{triggerLabel('tiktok.chat', [], locale)}</p>
            </div>

            <div class="plg-field">
              <span class="act-label">{t(locale, 'behavior.copy.colFilters')}</span>
              <p class="plg-note plg-mono">{MODERATION_BLOCKED_PATH} is true</p>
            </div>

            <div class="plg-field">
              <span class="act-label">{t(locale, 'behavior.copy.moderationPenaltyViewer')}</span>
              <p class="plg-note plg-mono">{MODERATION_VIEWER_TEMPLATE}</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
  };
  },
);

export default ModerationPenaltyEditor;
</script>
