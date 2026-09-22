<script lang="tsx">
import { ref } from 'vue';
import { Modal, ModalActions } from './ui/Modal.vue';
import { Button } from './ui/Button.vue';
import { defineVueComponent } from '../vue/component.ts';
import { t, type Locale } from '../i18n.ts';
import { errorMessage } from '../platform/control-client.ts';
import WidgetHost from '../../widgets/sdk/WidgetHost.vue';
import { widgetTemplates, type WidgetKind } from '../../widgets/sdk/templates.ts';
import { widgetStyleFields, type WidgetStyle } from '../../widgets/sdk/template.ts';

type Props = {
  locale: Locale;
  kind: WidgetKind;
  title: string;
  design: WidgetStyle;
  onSave: (kind: WidgetKind, design: WidgetStyle) => Promise<void>;
  onClose: () => void;
};

export default defineVueComponent<Props>(['locale', 'kind', 'title', 'design', 'onSave', 'onClose'], (props) => {
  const draft = ref<WidgetStyle>({ ...props.design });
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
      </div>
      {error.value ? <p role="alert">{error.value}</p> : null}
    </Modal>
  );
});
</script>

<style scoped>
.widget-style-editor { display: grid; grid-template-columns: minmax(0, 1fr) 210px; gap: 24px; }
.widget-style-editor__preview { height: 300px; }
.widget-style-editor__fields { border: 0; padding: 0; margin: 0; min-width: 0; }
.widget-style-editor__field { display: grid; grid-template-columns: 1fr auto; gap: 8px; margin-bottom: 20px; align-items: center; }
.widget-style-editor__field output { grid-column: 1 / -1; font-size: 12px; opacity: .7; }
.widget-style-editor__field input[type="color"] { width: 44px; height: 32px; padding: 2px; background: transparent; border: 1px solid #596070; border-radius: 6px; cursor: pointer; }
.widget-style-editor__field input[type="range"] { grid-column: 1 / -1; width: 100%; }
@media (max-width: 700px) { .widget-style-editor { grid-template-columns: minmax(0, 1fr); } }
</style>
