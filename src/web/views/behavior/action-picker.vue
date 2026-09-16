<script lang="tsx">
import { computed, ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { SearchInput } from '../../components/ui/TextInput.vue';
import { IconChevronLeft } from '../../components/icons/index.ts';
import type { ActionTypeDefinition, PluginStatus } from '../../../automation/behavior/types.ts';
import { i18nText, t, type Locale } from '../../i18n.ts';

type ActionPickerProps = {
  locale: Locale;
  plugins: PluginStatus[];
  actionTypes: ActionTypeDefinition[];
  onPick: (type: ActionTypeDefinition) => void;
  onCancel: () => void;
  onOpenPlugins: () => void;
};

export const ActionPicker = defineVueComponent<ActionPickerProps>(
  ['locale', 'plugins', 'actionTypes', 'onPick', 'onCancel', 'onOpenPlugins'],
  (props) => {
  const query = ref('');
  const matches = (type: ActionTypeDefinition): boolean =>
    !query.value.trim() || i18nText(props.locale, type.title).toLowerCase().includes(query.value.trim().toLowerCase());

  const installed = computed(() => props.plugins.filter((plugin) => plugin.installed && plugin.enabled));

  return () => (
    <div class="plg">
      <div class="plg-topbar">
        <button type="button" class="plg-btn plg-btn--icon" onClick={props.onCancel} aria-label={t(props.locale, 'behavior.copy.back')}><IconChevronLeft size={16} /></button>
        <div class="plg-topbar__text">
          <h2 class="plg-topbar__title">{t(props.locale, 'behavior.copy.pickTitle')}</h2>
          <span class="plg-topbar__subtitle">{t(props.locale, 'behavior.copy.pickLead')}</span>
        </div>
        <div class="plg-topbar__actions">
          <button type="button" class="plg-btn plg-btn--sm" onClick={props.onOpenPlugins}>{t(props.locale, 'behavior.copy.explore')}</button>
        </div>
      </div>

      <div class="plg-toolbar">
        <SearchInput
          name="actionSearch"
          value={query.value}
          onValueChange={(next) => { query.value = next; }}
          placeholder={t(props.locale, 'behavior.copy.searchAction')}
        />
      </div>

      <div class="plg-scroll">
        <div class="plg-section">
          <div class="plg-group-head">
            <span class="plg-section-title">{t(props.locale, 'behavior.copy.builtInGroup')}</span>
            <span class="plg-group-note">{t(props.locale, 'behavior.copy.builtInNote')}</span>
          </div>
          <div class="plg-cards">
            {props.actionTypes.filter((type) => type.source.kind === 'builtin').filter(matches).map((type) => (
              <ActionTypeCard key={type.id} locale={props.locale} type={type} onPick={() => props.onPick(type)} />
            ))}
          </div>
        </div>

        {installed.value.map((plugin) => {
          const types = props.actionTypes.filter((type) => type.source.kind === 'plugin' && type.source.pluginId === plugin.descriptor.id).filter(matches);
          if (types.length === 0) return null;
          return (
            <div class="plg-section" key={plugin.descriptor.id}>
              <div class="plg-group-head">
                <span class="plg-section-title">{i18nText(props.locale, plugin.descriptor.name)}</span>
                <span class="plg-pill">{t(props.locale, 'behavior.copy.pluginNote')}</span>
                <span class="plg-group-note">
                  {i18nText(props.locale, plugin.descriptor.dependency)} · {types.length}
                </span>
              </div>
              <div class="plg-cards">
                {types.map((type) => (
                  <ActionTypeCard key={type.id} locale={props.locale} type={type} onPick={() => props.onPick(type)} />
                ))}
              </div>
            </div>
          );
        })}

        <div class="plg-section">
          <div class="plg-advanced">
            <div class="plg-field">
              <span class="plg-action-card__title">{t(props.locale, 'behavior.copy.missingTitle')}</span>
              <span class="plg-action-card__desc">{t(props.locale, 'behavior.copy.missingDesc')}</span>
            </div>
            <button type="button" class="plg-btn plg-btn--primary plg-btn--sm" onClick={props.onOpenPlugins}>
              {t(props.locale, 'behavior.copy.explore')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
  },
);

function ActionTypeCard({
  locale,
  type,
  onPick,
}: {
  locale: Locale;
  type: ActionTypeDefinition;
  onPick: () => void;
}) {
  return (
    <button type="button" class="plg-action-card" onClick={onPick}>
      <span class="plg-action-card__head">
        <span class="plg-action-card__title">{i18nText(locale, type.title)}</span>
        <span class="plg-pill plg-pill--mono">{type.tag}</span>
      </span>
      <span class="plg-action-card__desc">{i18nText(locale, type.description)}</span>
    </button>
  );
}

export default ActionPicker;
</script>
