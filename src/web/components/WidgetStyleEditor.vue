<script lang="tsx">
import { ref, useId } from 'vue';
import { Modal, ModalActions } from './ui/Modal.vue';
import { Button } from './ui/Button.vue';
import { defineVueComponent } from '../vue/component.ts';
import { t, type Locale } from '../i18n.ts';
import { errorMessage } from '../platform/control-client.ts';
import WidgetHost from '../../widgets/sdk/WidgetHost.vue';
import { widgetTemplates, type WidgetKind } from '../../widgets/sdk/templates.ts';
import { widgetStyleFields, type WidgetStyle } from '../../widgets/sdk/template.ts';
import { textDefaults, type TextField } from '../../widgets/sdk/text.ts';

type Props = {
  locale: Locale;
  kind: WidgetKind;
  title: string;
  design: WidgetStyle;
  onSave: (kind: WidgetKind, design: WidgetStyle) => Promise<void>;
  onClose: () => void;
};

export default defineVueComponent<Props>(['locale', 'kind', 'title', 'design', 'onSave', 'onClose'], (props) => {
  const draft = ref<WidgetStyle>({ ...props.design, text: { ...props.design.text } });
  const activeTab = ref<'styles' | 'templates'>('styles');
  const tabId = useId();
  const textLabels = { title: 'widgetsTextTitle', streakTitle: 'widgetsTextStreak', name: 'widgetsTextName', handle: 'widgetsTextHandle', message: 'widgetsTextMessage', count: 'widgetsTextCount', diamonds: 'widgetsTextDiamonds' } as const;
  const defaultsText: Partial<Record<TextField, string>> = textDefaults[props.kind];
  const placeholders = props.kind === 'gift' ? '{{name}}, {{gift}}, {{count}}, {{diamonds}}'
    : props.kind === 'chat' ? '{{name}}, {{username}}, {{message}}' : '{{name}}, {{username}}';
  const saving = ref(false);
  const error = ref('');
  const replay = ref(0);
  const defaults = {
    background: props.kind === 'chat' ? '#16161ddb' : '#16161d',
    textColor: props.kind === 'chat' ? '#e8e8ec' : '#f5f5f7',
    accent: { follow: '#22c55e', gift: '#f5c518', chat: '#f5f5f7', share: '#22d3ee', subscribe: '#fe2c55' }[props.kind],
    radius: props.kind === 'chat' ? 12 : 16,
  };
  const close = () => { if (!saving.value) props.onClose(); };
  const save = async () => {
    saving.value = true;
    error.value = '';
    try { await props.onSave(props.kind, draft.value); props.onClose(); }
    catch (failure) { error.value = errorMessage(failure); }
    finally { saving.value = false; }
  };
  const labels = { background: 'widgetsDesignBackground', textColor: 'widgetsDesignText', accent: 'widgetsDesignAccent', radius: 'widgetsDesignRadius' } as const;
  return () => (
    <Modal title={`${t(props.locale, 'widgetsEditDesign')} · ${props.title}`} size="xl"
      description={t(props.locale, 'widgetsDesignObsNote')} onClose={close} closeOnBackdrop={false}
      closeOnEscape={!saving.value} closeLabel={t(props.locale, 'widgetsDesignCancel')}
      footer={<ModalActions>
        <Button variant="ghost" disabled={saving.value} onClick={() => { draft.value = {}; }}>{t(props.locale, 'widgetsDesignReset')}</Button>
        <Button variant="soft" disabled={saving.value} onClick={close}>{t(props.locale, 'widgetsDesignCancel')}</Button>
        <Button variant="primary" loading={saving.value} disabled={saving.value} onClick={() => { void save(); }}>{t(props.locale, 'widgetsDesignSave')}</Button>
      </ModalActions>}
    >
      <div class="widget-style-editor">
        <div>
          <div class="widgets-preview-frame widget-style-editor__preview">
            <WidgetHost template={widgetTemplates[props.kind]} mode="preview" design={draft.value} replayKey={replay.value} />
          </div>
          <Button variant="ghost" onClick={() => { replay.value++; }}>{t(props.locale, 'widgetsReplay')}</Button>
        </div>
        <div class="widget-style-editor__controls">
          <div class="widget-style-editor__tabs" role="tablist" aria-label={t(props.locale, 'widgetsEditDesign')}>
            {(['styles', 'templates'] as const).map((tab) => <button
              type="button" role="tab" id={`${tabId}-${tab}`} aria-controls={`${tabId}-panel`}
              aria-selected={activeTab.value === tab} tabindex={activeTab.value === tab ? 0 : -1}
              onClick={() => { activeTab.value = tab; }}
              onKeydown={(event) => {
                if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
                event.preventDefault();
                const next = event.key === 'Home' ? 'styles' : event.key === 'End' ? 'templates' : tab === 'styles' ? 'templates' : 'styles';
                activeTab.value = next;
                document.getElementById(`${tabId}-${next}`)?.focus();
              }}
            >{t(props.locale, tab === 'styles' ? 'widgetsStylesTab' : 'widgetsTemplatesTab')}</button>)}
          </div>
          <div role="tabpanel" id={`${tabId}-panel`} aria-labelledby={`${tabId}-${activeTab.value}`}>
          {activeTab.value === 'templates' ?
          <fieldset class="widget-style-editor__texts" disabled={saving.value} aria-label={t(props.locale, 'widgetsTextHeading')}>
            <p class="widget-style-editor__hint">{t(props.locale, 'widgetsTextHint')} <code>{placeholders}</code></p>
            {(Object.keys(defaultsText) as TextField[]).map((field) => <label class="widget-style-editor__text-field">
              <span>{t(props.locale, textLabels[field])}</span>
              <input type="text" maxlength={300} value={draft.value.text?.[field] ?? defaultsText[field]}
                onInput={(event) => {
                  draft.value.text = { ...draft.value.text, [field]: (event.target as HTMLInputElement).value };
                }} />
            </label>)}
          </fieldset>
          :
        <fieldset class="widget-style-editor__fields" disabled={saving.value}>
          {widgetStyleFields.map((field) => <label class="widget-style-editor__field">
            <span>{t(props.locale, labels[field.key])}</span>
            {field.type === 'color' ? <input type="color" value={(draft.value[field.key] ?? defaults[field.key]).slice(0, 7)}
              onInput={(event) => { draft.value[field.key] = (event.target as HTMLInputElement).value; }} />
              : <input type="range" min={field.min} max={field.max} value={draft.value.radius ?? defaults.radius}
                onInput={(event) => { draft.value.radius = Number((event.target as HTMLInputElement).value); }} />}
            <output>{draft.value[field.key] ?? defaults[field.key]}{field.type === 'number' ? ' px' : ''}</output>
          </label>)}
        </fieldset>
          }
          </div>
        </div>
      </div>
      {error.value ? <p role="alert">{error.value}</p> : null}
    </Modal>
  );
});
</script>

<style scoped>
.widget-style-editor { display: grid; grid-template-columns: minmax(0, 1fr) 280px; gap: 24px; }
.widget-style-editor__preview { height: 300px; }
.widget-style-editor__controls { min-width: 0; }
.widget-style-editor__tabs { display: flex; gap: 4px; border-bottom: 1px solid var(--line); margin-bottom: 16px; }
.widget-style-editor__tabs button { flex: 1; padding: 10px 8px; border: 0; border-bottom: 2px solid transparent; background: transparent; color: var(--text-muted); cursor: pointer; font: inherit; font-weight: 600; }
.widget-style-editor__tabs button[aria-selected="true"] { color: var(--text); border-bottom-color: #00dce8; background: rgba(0, 220, 232, .06); }
.widget-style-editor__tabs button:focus-visible { outline: 2px solid var(--text); outline-offset: -2px; }
.widget-style-editor__texts { display: grid; grid-template-columns: minmax(0, 1fr); border: 0; padding: 0; margin: 0; min-width: 0; }
.widget-style-editor__hint { grid-column: 1 / -1; font-size: 12px; color: var(--text-muted); line-height: 1.5; margin: 0 0 12px; }
.widget-style-editor__text-field { display: grid; gap: 6px; margin-bottom: 12px; font-size: 13px; }
.widget-style-editor__text-field input { width: 100%; box-sizing: border-box; border: 1px solid var(--line); border-radius: 6px; background: var(--panel-solid); color: var(--text); padding: 8px 10px; }
.widget-style-editor__fields { border: 0; padding: 0; margin: 0; min-width: 0; }
.widget-style-editor__field { display: grid; grid-template-columns: 1fr auto; gap: 8px; margin-bottom: 20px; align-items: center; }
.widget-style-editor__field output { grid-column: 1 / -1; font-size: 12px; opacity: .7; }
.widget-style-editor__field input[type="color"] { width: 44px; height: 32px; padding: 2px; background: transparent; border: 1px solid #596070; border-radius: 6px; cursor: pointer; }
.widget-style-editor__field input[type="range"] { grid-column: 1 / -1; width: 100%; }
@media (max-width: 700px) { .widget-style-editor { grid-template-columns: minmax(0, 1fr); } }
@media (max-width: 480px) { .widget-style-editor__texts { grid-template-columns: minmax(0, 1fr); } }
</style>
