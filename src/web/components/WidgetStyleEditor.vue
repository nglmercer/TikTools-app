<script lang="tsx">
import { computed, ref, useId, watch, type VNodeChild } from 'vue';
import { Modal } from './ui/Modal.vue';
import { Button } from './ui/Button.vue';
import {
  IconFormat,
  IconImage,
  IconMore,
  IconPlay,
  IconPlus,
  IconRedo,
  IconSquare,
  IconStar,
  IconUndo,
} from './icons/index.ts';
import { defineVueComponent } from '../vue/component.ts';
import { t, type Locale } from '../i18n.ts';
import { errorMessage } from '../platform/control-client.ts';
import WidgetHost from '../../widgets/sdk/WidgetHost.vue';
import { widgetTemplates, type WidgetKind } from '../../widgets/sdk/templates.ts';
import { defaultWidgetLayers, updateWidgetLayer } from '../../widgets/sdk/layers.ts';
import type { WidgetAlign, WidgetAvatarStyle, WidgetBadgeStyle, WidgetEditorControl, WidgetEditorSection, WidgetLayer, WidgetLayerSeed, WidgetStyle, WidgetTemplateToken } from '../../widgets/sdk/template.ts';
import { orderedTextFields, textDefaults, type TextField } from '../../widgets/sdk/text.ts';
import { cloneDesign, EditorHistory } from './widgets-editor/history.ts';

type Props = {
  locale: Locale;
  kind: WidgetKind;
  title: string;
  design: WidgetStyle;
  onSave: (kind: WidgetKind, design: WidgetStyle) => Promise<void>;
  onClose: () => void;
};

type BuilderComponent = WidgetEditorSection['id'];

const COMPONENT_ICON = { layers: IconFormat, text: IconFormat, card: IconSquare, image: IconImage, heading: IconStar } as const;
const COMPONENT_KEY = {
  layers: 'widgetsBuilderLayers', text: 'widgetsBuilderText', card: 'widgetsBuilderCard',
  image: 'widgetsBuilderImage', heading: 'widgetsBuilderHeading',
} as const;

const TOKEN_KEY = {
  name: 'widgetsTokenName', username: 'widgetsTokenUsername', gift: 'widgetsTokenGift',
  count: 'widgetsTokenCount', diamonds: 'widgetsTokenDiamonds', message: 'widgetsTokenMessage',
} as const;

const TEXT_LABEL_KEY = {
  title: 'widgetsTextTitle', streakTitle: 'widgetsTextStreak', name: 'widgetsTextName',
  handle: 'widgetsTextHandle', message: 'widgetsTextMessage', count: 'widgetsTextCount', diamonds: 'widgetsTextDiamonds',
} as const;

const HEX_PATTERN = /^#[0-9a-f]{6}([0-9a-f]{2})?$/i;

export default defineVueComponent<Props>(['locale', 'kind', 'title', 'design', 'onSave', 'onClose'], (props) => {
  const template = widgetTemplates[props.kind];
  const schema = template.schema;
  const draft = ref<WidgetStyle>(cloneDesign(props.design));
  const component = ref<BuilderComponent>('layers');
  const selectedLayer = ref<string | null>(null);
  const saving = ref(false);
  const error = ref('');
  const replay = ref(0);
  const canUndo = ref(false);
  const canRedo = ref(false);
  const tokenMenu = ref<TextField | null>(null);
  const resetMenu = ref(false);
  const addMenu = ref(false);
  const baseId = useId();

  type DraftSnapshot = { draft: WidgetStyle; visible: TextField[] };
  const history = new EditorHistory<DraftSnapshot>();
  let lastKey = '';
  let lastAt = 0;
  let textBefore: DraftSnapshot | null = null;
  const inputRefs: Partial<Record<TextField, HTMLInputElement | null>> = {};

  const defaultsText: Partial<Record<TextField, string>> = textDefaults[props.kind];
  const textFields = [...schema.textFields];
  const coreFields = [...schema.requiredTextFields];
  const tokens = [...schema.tokens];
  // The template schema is the source of truth for both screens. A saved
  // hiddenText list is the only reason a supported line is absent here.
  const visibleFields = ref<TextField[]>(textFields.filter((field) => !draft.value.hiddenText?.includes(field)));
  const orderedVisibleFields = computed(() => orderedTextFields(textFields, draft.value.textOrder)
    .filter((field) => visibleFields.value.includes(field)));
  const layerLabel = (seed: WidgetLayerSeed): string => seed.field
    ? t(props.locale, TEXT_LABEL_KEY[seed.field])
    : t(props.locale, seed.kind === 'art' ? 'widgetsBuilderGiftArt' : 'widgetsBuilderAvatar');
  const layers = computed(() => draft.value.layers ?? defaultWidgetLayers(props.kind, schema, draft.value, layerLabel));
  const rails = computed<BuilderComponent[]>(() => schema.editorSections
    .filter((section) => section.id !== 'heading' || section.textFields.some((field) => visibleFields.value.includes(field)))
    .map((section) => section.id));
  watch(rails, (sections) => {
    if (!sections.includes(component.value)) component.value = sections[0] ?? 'text';
  });

  const snapshot = (): DraftSnapshot => ({
    draft: cloneDesign(draft.value),
    visible: [...visibleFields.value],
  });

  const defaults = {
    background: schema.defaultDesign.background ?? '#16161d',
    textColor: schema.defaultDesign.textColor ?? '#f5f5f7',
    accent: schema.defaultDesign.accent ?? '#22c55e',
    radius: schema.defaultDesign.radius ?? 16,
    borderColor: schema.defaultDesign.borderColor ?? '#2a2a33',
    borderWidth: schema.defaultDesign.borderWidth ?? 1,
    opacity: schema.defaultDesign.opacity ?? 100,
    padding: schema.defaultDesign.padding ?? 16,
    gap: schema.defaultDesign.gap ?? 16,
    width: schema.defaultDesign.width ?? 440,
    avatarSize: schema.defaultDesign.avatar?.size ?? 56,
    badgeSize: schema.defaultDesign.badge?.fontSize ?? 11,
    badgeWeight: schema.defaultDesign.badge?.fontWeight ?? 700,
    badgeSpacing: schema.defaultDesign.badge?.letterSpacing ?? 1.5,
  };

  const syncHistory = (): void => {
    canUndo.value = history.canUndo;
    canRedo.value = history.canRedo;
  };

  const change = (key: string, mutate: () => void, coalesceMs = 0): void => {
    if (saving.value) return;
    const before = snapshot();
    mutate();
    const now = Date.now();
    if (!(coalesceMs > 0 && key === lastKey && now - lastAt < coalesceMs)) {
      history.commit(before);
    }
    lastKey = key;
    lastAt = now;
    syncHistory();
  };

  const undo = (): void => {
    const restored = history.undo(snapshot());
    if (restored) {
      draft.value = restored.draft;
      visibleFields.value = restored.visible;
    }
    syncHistory();
  };

  const redo = (): void => {
    const restored = history.redo(snapshot());
    if (restored) {
      draft.value = restored.draft;
      visibleFields.value = restored.visible;
    }
    syncHistory();
  };

  const close = (): void => {
    if (!saving.value) props.onClose();
  };

  const save = async (): Promise<void> => {
    if (saving.value) return;
    saving.value = true;
    error.value = '';
    try {
      await props.onSave(props.kind, cloneDesign(draft.value));
      props.onClose();
    } catch (failure) {
      error.value = errorMessage(failure);
    } finally {
      saving.value = false;
    }
  };

  const closeMenus = (): void => {
    tokenMenu.value = null;
    resetMenu.value = false;
    addMenu.value = false;
  };

  const updateAvatar = (key: keyof WidgetAvatarStyle, value: string | number | boolean | undefined): void => {
    const next = { ...(draft.value.avatar ?? {}) } as Record<string, string | number | boolean>;
    if (value === undefined) delete next[key];
    else next[key] = value;
    if (Object.keys(next).length === 0) {
      const rest = { ...draft.value };
      delete rest.avatar;
      draft.value = rest;
    } else {
      draft.value = { ...draft.value, avatar: next as WidgetAvatarStyle };
    }
  };

  const updateBadge = (key: keyof WidgetBadgeStyle, value: string | number | boolean | undefined): void => {
    const next = { ...(draft.value.badge ?? {}) } as Record<string, string | number | boolean>;
    if (value === undefined) delete next[key];
    else next[key] = value;
    if (Object.keys(next).length === 0) {
      const rest = { ...draft.value };
      delete rest.badge;
      draft.value = rest;
    } else {
      draft.value = { ...draft.value, badge: next as WidgetBadgeStyle };
    }
  };

  const setText = (field: TextField, value: string): void => {
    draft.value = { ...draft.value, text: { ...draft.value.text, [field]: value } };
  };

  const insertToken = (field: TextField, token: WidgetTemplateToken): void => {
    const input = inputRefs[field] ?? null;
    const current = draft.value.text?.[field] ?? defaultsText[field] ?? '';
    const caret = input?.selectionStart ?? current.length;
    const end = input?.selectionEnd ?? caret;
    const snippet = `{{${token}}}`;
    change(`text:${field}`, () => {
      setText(field, current.slice(0, caret) + snippet + current.slice(end));
    });
    tokenMenu.value = null;
    textBefore = null;
    requestAnimationFrame(() => {
      if (!input) return;
      input.focus();
      const position = caret + snippet.length;
      input.setSelectionRange(position, position);
    });
  };

  const resetSectionKeys = (): string[] => {
    const section = schema.editorSections.find((item) => item.id === component.value);
    if (!section) return [];
    const controlKeys: Partial<Record<WidgetEditorControl, string>> = {
      textColor: 'textColor', background: 'background', accent: 'accent', radius: 'radius',
      borderColor: 'borderColor', borderWidth: 'borderWidth', shadow: 'shadow', opacity: 'opacity',
      align: 'align', autoWidth: 'width', width: 'width', padding: 'padding', gap: 'gap',
      avatarVisible: 'avatar.visible', avatarSize: 'avatar.size', avatarRadius: 'avatar.radius',
      avatarBorderColor: 'avatar.borderColor', avatarBorderWidth: 'avatar.borderWidth',
      headingVisible: 'badge.visible', headingColor: 'badge.color', headingFontSize: 'badge.fontSize',
      headingWeight: 'badge.fontWeight', headingSpacing: 'badge.letterSpacing',
    };
    return [...new Set(section.groups.flatMap((group) => group.controls.flatMap((control) => {
      if (control === 'layers') return ['layers'];
      if (control === 'textFields') {
        return ['textOrder', ...textFields.map((field) => `text.${field}`)];
      }
      return controlKeys[control] ? [controlKeys[control]] : [];
    })))];
  };

  const setHidden = (hidden: TextField[]): void => {
    const next = { ...draft.value };
    if (hidden.length === 0) delete next.hiddenText;
    else next.hiddenText = textFields.filter((field) => hidden.includes(field));
    draft.value = next;
  };

  const removeField = (field: TextField): void => {
    // Removing a field hides its line and drops its customization.
    change(`text:remove:${field}`, () => {
      const text = { ...draft.value.text };
      delete text[field];
      draft.value = { ...draft.value, text };
      setHidden([...(draft.value.hiddenText ?? []), field]);
      visibleFields.value = visibleFields.value.filter((item) => item !== field);
    });
  };

  const addField = (field: TextField, placement: 'top' | 'bottom'): void => {
    change('field:add', () => {
      const order = orderedTextFields(textFields, draft.value.textOrder).filter((item) => item !== field);
      const visible = order.filter((item) => visibleFields.value.includes(item));
      const hidden = order.filter((item) => !visibleFields.value.includes(item));
      draft.value = { ...draft.value, textOrder: placement === 'top'
        ? [field, ...visible, ...hidden]
        : [...visible, field, ...hidden] };
      visibleFields.value = [...visibleFields.value, field];
      setHidden((draft.value.hiddenText ?? []).filter((item) => item !== field));
    });
    addMenu.value = false;
  };

  const moveField = (field: TextField, direction: -1 | 1): void => {
    const visible = orderedVisibleFields.value;
    const index = visible.indexOf(field);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= visible.length) return;
    change('text:order', () => {
      const next = [...visible];
      [next[index], next[target]] = [next[target]!, next[index]!];
      const hidden = orderedTextFields(textFields, draft.value.textOrder)
        .filter((item) => !visibleFields.value.includes(item));
      draft.value = { ...draft.value, textOrder: [...next, ...hidden] };
    });
  };

  const setLayer = (id: string, patch: Partial<WidgetLayer>): void => {
    draft.value = { ...draft.value, layers: updateWidgetLayer(layers.value, id, (layer) => ({ ...layer, ...patch })) };
  };

  const editLayer = (id: string, key: string, patch: Partial<WidgetLayer>): void => {
    change(`layer:${id}:${key}`, () => setLayer(id, patch), 700);
  };

  const moveLayer = (id: string, direction: -1 | 1): void => {
    const current = layers.value;
    const index = current.findIndex((layer) => layer.id === id);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= current.length) return;
    change('layers:move', () => {
      const next = [...current];
      [next[index], next[target]] = [next[target]!, next[index]!];
      draft.value = { ...draft.value, layers: next };
    });
  };

  const removeLayer = (id: string): void => {
    change('layers:remove', () => {
      draft.value = { ...draft.value, layers: layers.value.filter((layer) => layer.id !== id) };
    });
    if (selectedLayer.value === id) selectedLayer.value = null;
  };

  const addLayer = (kind: WidgetLayer['kind'], placement: 'top' | 'bottom'): void => {
    const label = t(props.locale, kind === 'text' ? 'widgetsBuilderText' : kind === 'art' ? 'widgetsBuilderGiftArt' : 'widgetsBuilderAvatar');
    let number = 1;
    while (layers.value.some((layer) => layer.name === `${label} ${number}`)) number++;
    let id: string;
    do { id = `layer:${Math.random().toString(36).slice(2, 12)}`; }
    while (layers.value.some((layer) => layer.id === id));
    const layer: WidgetLayer = { id, kind, name: `${label} ${number}` };
    if (kind === 'text') layer.text = t(props.locale, 'widgetsBuilderNewText');
    else layer.size = kind === 'art' ? 88 : defaults.avatarSize;
    change('layers:add', () => {
      const current = layers.value;
      draft.value = { ...draft.value, layers: placement === 'top' ? [layer, ...current] : [...current, layer] };
    });
    selectedLayer.value = id;
    addMenu.value = false;
  };

  const resetSection = (): void => {
    const keys = resetSectionKeys();
    change('reset', () => {
      const next = { ...draft.value };
      for (const key of keys) {
        const [parent, child] = key.split('.');
        if (parent && child && (parent === 'text' || parent === 'avatar' || parent === 'badge')) {
          const nested = next[parent];
          if (!nested) continue;
          const updated = { ...nested } as Record<string, unknown>;
          delete updated[child];
          if (Object.keys(updated).length > 0) {
            (next as Record<string, unknown>)[parent] = updated;
          } else {
            delete next[parent];
          }
        } else {
          delete next[key as keyof WidgetStyle];
        }
      }
      draft.value = next;
      const resetTextFields = textFields.filter((field) => keys.includes(`text.${field}`));
      if (resetTextFields.length > 0) {
        // Resetting text restores the template's visibility. Other explicit
        // hidden fields remain hidden until the user adds them back.
        const hidden = new Set(draft.value.hiddenText ?? []);
        resetTextFields.forEach((field) => hidden.delete(field));
        setHidden([...hidden]);
        visibleFields.value = textFields.filter((field) => !hidden.has(field));
      }
    });
    resetMenu.value = false;
  };

  const resetDesign = (): void => {
    // Reset restores the schema defaults: every default template line is back.
    change('reset', () => {
      draft.value = {};
      visibleFields.value = [...textFields];
    });
    resetMenu.value = false;
  };

  const onRootKeydown = (event: KeyboardEvent): void => {
    if (event.key === 'Escape' && (tokenMenu.value !== null || resetMenu.value || addMenu.value)) {
      event.stopPropagation();
      closeMenus();
      return;
    }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'z' && !saving.value) {
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) return;
      event.preventDefault();
      if (event.shiftKey) redo();
      else undo();
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'y' && !saving.value) {
      event.preventDefault();
      redo();
    }
  };

  const colorValue = (value: string | undefined, fallback: string): string =>
    value && HEX_PATTERN.test(value) ? value.slice(0, 7) : fallback.slice(0, 7);

  const renderColorRow = (label: string, value: string | undefined, fallback: string, key: string, apply: (color: string) => void) => (
    <label class="wb-row wb-color">
      <span class="wb-label">{label}</span>
      <span class="wb-color__inputs">
        <input
          type="color"
          value={colorValue(value, fallback)}
          disabled={saving.value}
          aria-label={label}
          onInput={(event) => change(key, () => apply((event.target as HTMLInputElement).value), 800)}
        />
        <input
          type="text"
          class="wb-hex"
          value={value ?? fallback}
          maxlength={9}
          spellcheck={false}
          disabled={saving.value}
          aria-label={`${label} hex`}
          onChange={(event) => {
            const input = event.target as HTMLInputElement;
            const next = input.value.trim();
            if (HEX_PATTERN.test(next)) change(key, () => apply(next));
            else input.value = value ?? fallback;
          }}
        />
      </span>
    </label>
  );

  const renderSliderRow = (
    label: string, value: number, min: number, max: number, step: number, unit: string, key: string,
    apply: (next: number) => void, disabled = false,
  ) => (
    <label class="wb-row wb-slider">
      <span class="wb-label">{label}</span>
      <span class="wb-slider__control">
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          disabled={disabled || saving.value}
          aria-label={label}
          onInput={(event) => change(key, () => apply(Number((event.target as HTMLInputElement).value)), 700)}
        />
        <output>{value}{unit}</output>
      </span>
    </label>
  );

  const renderToggleRow = (label: string, checked: boolean, key: string, apply: (next: boolean) => void) => (
    <div class="wb-row wb-toggle">
      <span class="wb-label" id={`${baseId}-${key}`}>{label}</span>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-labelledby={`${baseId}-${key}`}
        class={['wb-switch', checked ? 'is-on' : '']}
        disabled={saving.value}
        onClick={() => change(key, () => apply(!checked))}
      >
        <span class="wb-switch__thumb" aria-hidden="true" />
      </button>
    </div>
  );

  const renderTextField = (field: TextField, removable = false) => {
    const label = t(props.locale, TEXT_LABEL_KEY[field]);
    const menuOpen = tokenMenu.value === field;
    const inputId = `${baseId}-text-${field}`;
    const position = orderedVisibleFields.value.indexOf(field);
    return (
      <div class="wb-text" key={field}>
        <span class="wb-label-row">
          <label class="wb-label" for={inputId}>{label}</label>
          <span class="wb-layer-actions">
            <button type="button" class="wb-layer-action" disabled={position <= 0 || saving.value}
              aria-label={`${label}: ${t(props.locale, 'widgetsBuilderMoveUp')}`}
              onClick={() => moveField(field, -1)}>↑</button>
            <button type="button" class="wb-layer-action" disabled={position >= orderedVisibleFields.value.length - 1 || saving.value}
              aria-label={`${label}: ${t(props.locale, 'widgetsBuilderMoveDown')}`}
              onClick={() => moveField(field, 1)}>↓</button>
            {removable ? (
              <button type="button" class="wb-remove" disabled={saving.value}
                aria-label={`${label}: ${t(props.locale, 'widgetsBuilderRemoveField')}`}
                onClick={() => removeField(field)}>×</button>
            ) : null}
          </span>
        </span>
        <span class="wb-text__input">
            <input
              ref={(element) => { inputRefs[field] = element as HTMLInputElement | null; }}
              id={inputId}
              type="text"
              maxlength={300}
              value={draft.value.text?.[field] ?? defaultsText[field] ?? ''}
              disabled={saving.value}
              onFocus={() => { textBefore = snapshot(); }}
              onInput={(event) => setText(field, (event.target as HTMLInputElement).value)}
              onChange={() => {
                if (textBefore && JSON.stringify(textBefore.draft) !== JSON.stringify(draft.value)) {
                  history.commit(textBefore);
                  syncHistory();
                }
                textBefore = null;
              }}
            />
            <button
              type="button"
              class={['wb-token-btn', menuOpen ? 'is-open' : '']}
              disabled={saving.value}
              aria-label={t(props.locale, 'widgetsBuilderInsertToken')}
              aria-expanded={menuOpen}
              aria-haspopup="menu"
              onClick={() => { tokenMenu.value = menuOpen ? null : field; }}
            >
              {'{+}'}
            </button>
        </span>
        {menuOpen ? (
          <div class="wb-menu" role="menu" aria-label={t(props.locale, 'widgetsBuilderInsertToken')}>
            {tokens.map((token) => (
              <button type="button" role="menuitem" onClick={() => insertToken(field, token)}>
                <code>{`{{${token}}}`}</code>
                <span>{t(props.locale, TOKEN_KEY[token])}</span>
              </button>
            ))}
          </div>
        ) : null}
      </div>
    );
  };

  const renderAlignRow = () => (
    <div class="wb-row wb-align">
      <span class="wb-label" id={`${baseId}-align`}>{t(props.locale, 'widgetsBuilderAlign')}</span>
      <div class="wb-segmented" role="radiogroup" aria-labelledby={`${baseId}-align`}>
        {(['left', 'center', 'right'] as const).map((option) => (
          <button
            type="button"
            role="radio"
            aria-checked={(draft.value.align ?? 'left') === option}
            class={(draft.value.align ?? 'left') === option ? 'is-active' : ''}
            disabled={saving.value}
            onClick={() => change('align', () => {
              if (option === 'left') {
                const next = { ...draft.value };
                delete next.align;
                draft.value = next;
              } else {
                draft.value = { ...draft.value, align: option as WidgetAlign };
              }
            })}
          >
            {t(props.locale, option === 'left' ? 'widgetsBuilderAlignLeft' : option === 'center' ? 'widgetsBuilderAlignCenter' : 'widgetsBuilderAlignRight')}
          </button>
        ))}
      </div>
    </div>
  );

  const renderAddField = () => {
    const hidden = textFields.filter((field) => !visibleFields.value.includes(field));
    if (hidden.length === 0) return null;
    return (
      <div class="wb-add">
        <button
          type="button"
          class="wb-add__btn"
          disabled={saving.value}
          aria-expanded={addMenu.value}
          aria-haspopup="menu"
          onClick={() => { addMenu.value = !addMenu.value; }}
        >
          <IconPlus size={14} />
          <span>{t(props.locale, 'widgetsBuilderAddField')}</span>
        </button>
        {addMenu.value ? (
          <div class="wb-menu" role="menu" aria-label={t(props.locale, 'widgetsBuilderAddField')}>
            {hidden.map((field) => (
              <div class="wb-add__choice">
                <span>{t(props.locale, TEXT_LABEL_KEY[field])}</span>
                <span>
                  <button type="button" role="menuitem" onClick={() => addField(field, 'top')}
                    aria-label={`${t(props.locale, TEXT_LABEL_KEY[field])}: ${t(props.locale, 'widgetsBuilderAddTop')}`}>
                    {t(props.locale, 'widgetsBuilderAddTop')}
                  </button>
                  <button type="button" role="menuitem" onClick={() => addField(field, 'bottom')}
                    aria-label={`${t(props.locale, TEXT_LABEL_KEY[field])}: ${t(props.locale, 'widgetsBuilderAddBottom')}`}>
                    {t(props.locale, 'widgetsBuilderAddBottom')}
                  </button>
                </span>
              </div>
            ))}
          </div>
        ) : null}
      </div>
    );
  };

  const renderLayers = () => (
    <div class="wb-layer-editor">
      <p class="wb-layer-editor__hint">{t(props.locale, 'widgetsBuilderLayerHint')}</p>
      <div class="wb-layer-list">
        {layers.value.map((layer, index) => {
          const expanded = selectedLayer.value === layer.id;
          return <div class={['wb-layer', expanded ? 'is-expanded' : '']} key={layer.id}>
            <div class="wb-layer__header">
              <button type="button" class="wb-layer__select" aria-expanded={expanded}
                onClick={() => { selectedLayer.value = expanded ? null : layer.id; }}>
                <span class="wb-layer__number">{index + 1}</span>
                <span class="wb-layer__name">{layer.name}</span>
              </button>
              <span class="wb-layer-actions">
                <button type="button" disabled={index === 0 || saving.value}
                  aria-label={`${layer.name}: ${t(props.locale, 'widgetsBuilderMoveUp')}`}
                  onClick={() => moveLayer(layer.id, -1)}>↑</button>
                <button type="button" disabled={index === layers.value.length - 1 || saving.value}
                  aria-label={`${layer.name}: ${t(props.locale, 'widgetsBuilderMoveDown')}`}
                  onClick={() => moveLayer(layer.id, 1)}>↓</button>
                <button type="button" disabled={saving.value}
                  aria-label={`${layer.name}: ${t(props.locale, 'widgetsBuilderRemoveLayer')}`}
                  onClick={() => removeLayer(layer.id)}>×</button>
              </span>
            </div>
            {expanded ? <div class="wb-layer__details">
              <label class="wb-row">
                <span class="wb-label">{t(props.locale, 'widgetsBuilderLayerName')}</span>
                <input class="wb-layer__input" type="text" maxlength={80} value={layer.name}
                  disabled={saving.value}
                  onInput={(event) => editLayer(layer.id, 'name', { name: (event.target as HTMLInputElement).value })} />
              </label>
              {layer.kind === 'text' ? <>
                <label class="wb-row">
                  <span class="wb-label">{t(props.locale, 'widgetsBuilderLayerContent')}</span>
                  <textarea class="wb-layer__input" rows={3} maxlength={300} value={layer.text ?? ''}
                    disabled={saving.value}
                    onInput={(event) => editLayer(layer.id, 'text', { text: (event.target as HTMLTextAreaElement).value })} />
                </label>
                <div class="wb-layer__tokens" aria-label={t(props.locale, 'widgetsBuilderInsertToken')}>
                  {tokens.map((token) => <button type="button" key={token}
                    disabled={saving.value || (layer.text ?? '').length + token.length + 4 > 300}
                    onClick={() => editLayer(layer.id, 'text', { text: `${layer.text ?? ''}{{${token}}}` })}>
                    {t(props.locale, TOKEN_KEY[token])}
                  </button>)}
                </div>
                {renderColorRow(t(props.locale, 'widgetsBuilderColor'), layer.color, defaults.textColor, `layer:${layer.id}:color`,
                  (color) => setLayer(layer.id, { color }))}
                {renderSliderRow(t(props.locale, 'widgetsBuilderFontSize'), layer.fontSize ?? (layer.field === 'name' ? 27 : 15),
                  8, 96, 1, ' px', `layer:${layer.id}:fontSize`, (size) => setLayer(layer.id, { fontSize: size }))}
                {renderSliderRow(t(props.locale, 'widgetsBuilderWeight'), layer.fontWeight ?? (layer.field === 'name' ? 800 : 400),
                  100, 900, 100, '', `layer:${layer.id}:fontWeight`, (weight) => setLayer(layer.id, { fontWeight: weight }))}
              </> : renderSliderRow(t(props.locale, 'widgetsBuilderSize'), layer.size ?? (layer.kind === 'art' ? 88 : defaults.avatarSize),
                16, 160, 1, ' px', `layer:${layer.id}:size`, (size) => setLayer(layer.id, { size }))}
            </div> : null}
          </div>;
        })}
      </div>
      <div class="wb-add">
        <button type="button" class="wb-add__btn" disabled={saving.value}
          aria-expanded={addMenu.value} aria-haspopup="menu"
          onClick={() => { addMenu.value = !addMenu.value; }}>
          <IconPlus size={14} /> {t(props.locale, 'widgetsBuilderAddLayer')}
        </button>
        {addMenu.value ? <div class="wb-menu wb-layer-add" role="menu" aria-label={t(props.locale, 'widgetsBuilderAddLayer')}>
          {(['text', 'avatar', ...(props.kind === 'gift' ? ['art'] : [])] as WidgetLayer['kind'][]).map((kind) => (
            <div class="wb-add__choice" key={kind}>
              <span>{t(props.locale, kind === 'text' ? 'widgetsBuilderText' : kind === 'art' ? 'widgetsBuilderGiftArt' : 'widgetsBuilderAvatar')}</span>
              <span>
                <button type="button" role="menuitem" onClick={() => addLayer(kind, 'top')}>{t(props.locale, 'widgetsBuilderAddTop')}</button>
                <button type="button" role="menuitem" onClick={() => addLayer(kind, 'bottom')}>{t(props.locale, 'widgetsBuilderAddBottom')}</button>
              </span>
            </div>
          ))}
        </div> : null}
      </div>
    </div>
  );

  const renderControl = (control: WidgetEditorControl): VNodeChild => {
    const d = draft.value;
    const autoWidth = d.width === undefined;
    switch (control) {
      case 'layers': return renderLayers();
      case 'textFields': {
        return orderedVisibleFields.value
          .map((field) => renderTextField(field, !coreFields.includes(field)));
      }
      case 'addTextField': return renderAddField();
      case 'textColor': return renderColorRow(t(props.locale, 'widgetsDesignText'), d.textColor, defaults.textColor, 'color:textColor',
        (color) => { draft.value = { ...draft.value, textColor: color }; });
      case 'background': return renderColorRow(t(props.locale, 'widgetsDesignBackground'), d.background, defaults.background, 'color:background',
        (color) => { draft.value = { ...draft.value, background: color }; });
      case 'accent': return renderColorRow(t(props.locale, 'widgetsDesignAccent'), d.accent, defaults.accent, 'color:accent',
        (color) => { draft.value = { ...draft.value, accent: color }; });
      case 'radius': return renderSliderRow(t(props.locale, 'widgetsDesignRadius'), d.radius ?? defaults.radius, 0, 48, 1, ' px', 'radius',
        (next) => { draft.value = { ...draft.value, radius: next }; });
      case 'borderColor': return renderColorRow(t(props.locale, 'widgetsBuilderBorder'), d.borderColor, defaults.borderColor, 'color:borderColor',
        (color) => { draft.value = { ...draft.value, borderColor: color }; });
      case 'borderWidth': return renderSliderRow(t(props.locale, 'widgetsBuilderBorderWidth'), d.borderWidth ?? defaults.borderWidth, 0, 8, 1, ' px', 'borderWidth',
        (next) => { draft.value = { ...draft.value, borderWidth: next }; });
      case 'shadow': return renderToggleRow(t(props.locale, 'widgetsBuilderShadow'), d.shadow !== false, 'shadow',
        (next) => {
          const nextDraft = { ...draft.value };
          if (next) delete nextDraft.shadow;
          else nextDraft.shadow = false;
          draft.value = nextDraft;
        });
      case 'opacity': return renderSliderRow(t(props.locale, 'widgetsBuilderOpacity'), d.opacity ?? defaults.opacity, 0, 100, 1, '%', 'opacity',
        (next) => { draft.value = { ...draft.value, opacity: next }; });
      case 'align': return renderAlignRow();
      case 'autoWidth': return renderToggleRow(t(props.locale, 'widgetsBuilderAuto'), autoWidth, 'width.auto',
        (next) => {
          const nextDraft = { ...draft.value };
          if (next) delete nextDraft.width;
          else nextDraft.width = defaults.width;
          draft.value = nextDraft;
        });
      case 'width': return renderSliderRow(t(props.locale, 'widgetsBuilderWidth'), d.width ?? defaults.width, 240, 720, 4, ' px', 'width',
        (next) => { draft.value = { ...draft.value, width: next }; }, autoWidth);
      case 'padding': return renderSliderRow(t(props.locale, 'widgetsBuilderPadding'), d.padding ?? defaults.padding, 0, 64, 1, ' px', 'padding',
        (next) => { draft.value = { ...draft.value, padding: next }; });
      case 'gap': return renderSliderRow(t(props.locale, 'widgetsBuilderGap'), d.gap ?? defaults.gap, 0, 48, 1, ' px', 'gap',
        (next) => { draft.value = { ...draft.value, gap: next }; });
      case 'avatarVisible': return renderToggleRow(t(props.locale, 'widgetsBuilderVisible'), d.avatar?.visible !== false, 'avatar.visible',
        (next) => updateAvatar('visible', next ? undefined : false));
      case 'avatarSize': return renderSliderRow(t(props.locale, 'widgetsBuilderSize'), d.avatar?.size ?? defaults.avatarSize, 16, 160, 1, ' px', 'avatar.size',
        (next) => updateAvatar('size', next));
      case 'avatarRadius': return renderSliderRow(t(props.locale, 'widgetsBuilderRound'), d.avatar?.radius ?? Math.min(80, (d.avatar?.size ?? defaults.avatarSize) / 2), 0, 80, 1, ' px', 'avatar.radius',
        (next) => updateAvatar('radius', next));
      case 'avatarBorderColor': return renderColorRow(t(props.locale, 'widgetsBuilderBorder'), d.avatar?.borderColor, defaults.accent, 'avatar.borderColor',
        (color) => updateAvatar('borderColor', color));
      case 'avatarBorderWidth': return renderSliderRow(t(props.locale, 'widgetsBuilderBorderWidth'), d.avatar?.borderWidth ?? 0, 0, 8, 1, ' px', 'avatar.borderWidth',
        (next) => updateAvatar('borderWidth', next));
      case 'headingVisible': return renderToggleRow(t(props.locale, 'widgetsBuilderVisible'), d.badge?.visible !== false, 'badge.visible',
        (next) => updateBadge('visible', next ? undefined : false));
      case 'headingColor': return renderColorRow(t(props.locale, 'widgetsBuilderColor'), d.badge?.color, defaults.accent, 'badge.color',
        (color) => updateBadge('color', color));
      case 'headingFontSize': return renderSliderRow(t(props.locale, 'widgetsBuilderFontSize'), d.badge?.fontSize ?? defaults.badgeSize, 8, 32, 1, ' px', 'badge.fontSize',
        (next) => updateBadge('fontSize', next));
      case 'headingWeight': return renderSliderRow(t(props.locale, 'widgetsBuilderWeight'), d.badge?.fontWeight ?? defaults.badgeWeight, 400, 900, 100, '', 'badge.fontWeight',
        (next) => updateBadge('fontWeight', next));
      case 'headingSpacing': return renderSliderRow(t(props.locale, 'widgetsBuilderSpacing'), d.badge?.letterSpacing ?? defaults.badgeSpacing, 0, 8, 0.5, ' px', 'badge.letterSpacing',
        (next) => updateBadge('letterSpacing', next));
    }
  };

  const renderPanel = () => {
    const section = schema.editorSections.find((item) => item.id === component.value);
    if (!section) return null;
    const groupTitles = {
      fields: 'widgetsBuilderFields',
      textAppearance: 'widgetsBuilderTextAppearance',
      appearance: 'widgetsBuilderAppearance',
      layout: 'widgetsBuilderLayout',
    } as const;
    return <div class="wb-groups">
      {section.groups.map((group) => {
        const controls = group.controls.map(renderControl);
        return group.title
          ? <section class="wb-group">
              <h3 class="wb-group__title">{t(props.locale, groupTitles[group.title])}</h3>
              <div class="wb-stack">{controls}</div>
            </section>
          : <div class="wb-stack">{controls}</div>;
      })}
    </div>;
  };

  return () => (
    <Modal
      title={`${t(props.locale, 'widgetsBuilder')} / ${props.title}`}
      size="xl"
      className="widget-builder-modal"
      onClose={close}
      closeOnBackdrop={false}
      closeOnEscape={!saving.value}
      closeLabel={t(props.locale, 'widgetsDesignCancel')}
      headerActions={[
        <Button
          variant="ghost"
          size="sm"
          icon={<IconUndo size={15} />}
          iconOnly
          tooltip={t(props.locale, 'widgetsBuilderUndo')}
          disabled={!canUndo.value || saving.value}
          onClick={undo}
        />,
        <Button
          variant="ghost"
          size="sm"
          icon={<IconRedo size={15} />}
          iconOnly
          tooltip={t(props.locale, 'widgetsBuilderRedo')}
          disabled={!canRedo.value || saving.value}
          onClick={redo}
        />,
        <Button
          variant="primary"
          size="sm"
          disabled={saving.value}
          onClick={() => { void save(); }}
        >
          {saving.value ? t(props.locale, 'widgetsBuilderSaving') : t(props.locale, 'widgetsBuilderSave')}
        </Button>,
      ]}
    >
      <div class="widget-builder" onKeydown={onRootKeydown}>
        <nav class="wb-rail" aria-label={t(props.locale, 'widgetsBuilder')}>
          <div class="wb-rail__heading">{t(props.locale, 'widgetsBuilderElements')}</div>
          {rails.value.map((item) => {
            const Icon = COMPONENT_ICON[item];
            const selected = component.value === item;
            return (
              <button
                type="button"
                class={['wb-rail__item', selected ? 'is-active' : '']}
                aria-pressed={selected}
                onClick={() => {
                  component.value = item;
                  closeMenus();
                }}
              >
                <Icon size={20} />
                <span>{t(props.locale, COMPONENT_KEY[item])}</span>
              </button>
            );
          })}
        </nav>

        <section class="wb-canvas" aria-label={t(props.locale, 'widgetsPreview')}>
          <div class="wb-canvas__toolbar">
            <div class="wb-canvas__title">
              <span class="wb-canvas__status" aria-hidden="true" />
              <span>{t(props.locale, 'widgetsBuilderPreview')}</span>
            </div>
            <span class="wb-canvas__hint">{t(props.locale, 'widgetsBuilderPreviewHint')}</span>
          </div>
          <div class="wb-canvas__workarea">
            <div class="wb-canvas__frame">
              <WidgetHost template={template} mode="preview" design={draft.value} replayKey={replay.value} />
            </div>
          </div>
          <div class="wb-canvas__footer">
            <span>{t(props.locale, 'widgetsBuilderCanvasSize')}</span>
            <Button
              variant="soft"
              size="sm"
              icon={<IconPlay size={14} />}
              tooltip={t(props.locale, 'widgetsReplay')}
              onClick={() => { replay.value++; }}
            >{t(props.locale, 'widgetsReplay')}</Button>
          </div>
        </section>

        <aside class="wb-inspector">
          <div class="wb-inspector__head">
            <div class="wb-inspector__selection">
              <span class="wb-inspector__icon">{(() => { const Icon = COMPONENT_ICON[component.value]; return <Icon size={18} />; })()}</span>
              <span class="wb-inspector__selection-text">
                <strong>{t(props.locale, COMPONENT_KEY[component.value])}</strong>
              </span>
            </div>
            <div class="wb-reset">
              <button
                type="button"
                class="wb-reset__btn"
                aria-label={t(props.locale, 'widgetsBuilderResetMenu')}
                aria-expanded={resetMenu.value}
                aria-haspopup="menu"
                disabled={saving.value}
                onClick={() => { resetMenu.value = !resetMenu.value; }}
              >
                <IconMore size={16} />
              </button>
              {resetMenu.value ? (
                <div class="wb-menu wb-menu--right" role="menu" aria-label={t(props.locale, 'widgetsBuilderResetMenu')}>
                  <button type="button" role="menuitem" onClick={resetSection}>
                    {t(props.locale, 'widgetsBuilderResetSection')}
                  </button>
                  <button type="button" role="menuitem" onClick={resetDesign}>
                    {t(props.locale, 'widgetsBuilderResetDesign')}
                  </button>
                </div>
              ) : null}
            </div>
          </div>
          <div
            class="wb-panel"
            id={`${baseId}-panel`}
            role="region"
            aria-label={t(props.locale, COMPONENT_KEY[component.value])}
          >
            <fieldset class="wb-fields" disabled={saving.value}>
              {renderPanel()}
            </fieldset>
          </div>
          {error.value ? <p class="wb-error" role="alert">{error.value}</p> : null}
        </aside>

        {tokenMenu.value !== null || resetMenu.value || addMenu.value ? (
          <div class="wb-scrim" aria-hidden="true" onMousedown={closeMenus} />
        ) : null}
      </div>
    </Modal>
  );
});
</script>

<style scoped>
.widget-builder {
  position: relative;
  display: grid;
  grid-template-columns: 154px minmax(0, 1fr) 328px;
  gap: 0;
  height: 100%;
  min-height: 0;
  font-size: 13px;
}

.wb-rail {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 22px 12px;
  border-right: 1px solid var(--line);
  background: color-mix(in srgb, var(--panel-solid) 88%, var(--bg));
  overflow-y: auto;
}

.wb-rail__heading {
  padding: 0 10px 12px;
  color: var(--text-muted);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.13em;
}

.wb-rail__item {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: flex-start;
  gap: 12px;
  min-height: 43px;
  padding: 9px 12px;
  border: 1px solid transparent;
  border-radius: 10px;
  background: transparent;
  color: var(--text-muted);
  font: inherit;
  font-size: 12px;
  font-weight: 650;
  text-align: left;
  cursor: pointer;
  transition: background 140ms ease, color 140ms ease;
}

.wb-rail__item:hover {
  background: var(--panel);
  color: var(--text);
}

.wb-rail__item.is-active {
  border-color: color-mix(in srgb, var(--tt-cyan) 26%, transparent);
  background: color-mix(in srgb, var(--tt-cyan) 11%, transparent);
  color: var(--text);
}

.wb-rail__item.is-active svg {
  color: var(--tt-cyan);
}

.wb-rail__item:focus-visible {
  outline: 2px solid #00dce8;
  outline-offset: 1px;
}

.wb-canvas {
  min-width: 0;
  min-height: 0;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  background: color-mix(in srgb, var(--bg) 65%, var(--panel-solid));
}

.wb-canvas__toolbar,
.wb-canvas__footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 54px;
  padding: 0 22px;
}

.wb-canvas__toolbar {
  border-bottom: 1px solid var(--line);
}

.wb-canvas__title {
  display: inline-flex;
  align-items: center;
  gap: 9px;
  color: var(--text);
  font-size: 12px;
  font-weight: 750;
}

.wb-canvas__status {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--tt-green);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--tt-green) 13%, transparent);
}

.wb-canvas__hint,
.wb-canvas__footer > span {
  color: var(--text-muted);
  font-size: 11px;
}

.wb-canvas__workarea {
  display: flex;
  min-width: 0;
  min-height: 0;
  align-items: center;
  justify-content: center;
  padding: 22px;
  overflow: hidden;
}

.wb-canvas__frame {
  position: relative;
  width: min(100%, calc((100dvh - 270px) * 16 / 9));
  max-height: 100%;
  aspect-ratio: 16 / 9;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--text) 14%, transparent);
  border-radius: 12px;
  background:
    repeating-conic-gradient(#20232b 0% 25%, #181b22 0% 50%) 0 0 / 24px 24px;
  box-shadow: 0 18px 48px rgba(0, 0, 0, 0.32), 0 0 0 8px rgba(255, 255, 255, 0.018);
}

.wb-canvas__footer {
  min-height: 57px;
  border-top: 1px solid var(--line);
}

.wb-canvas__footer .ui-btn {
  gap: 7px;
}

.wb-inspector {
  display: flex;
  flex-direction: column;
  min-height: 0;
  border-left: 1px solid var(--line);
  background: var(--panel-solid);
}

.wb-inspector__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 76px;
  padding: 14px 18px;
  border-bottom: 1px solid var(--line);
}

.wb-inspector__selection {
  display: flex;
  align-items: center;
  gap: 11px;
  min-width: 0;
}

.wb-inspector__icon {
  display: grid;
  width: 36px;
  height: 36px;
  flex: none;
  place-items: center;
  border: 1px solid color-mix(in srgb, var(--tt-cyan) 22%, transparent);
  border-radius: 10px;
  background: color-mix(in srgb, var(--tt-cyan) 10%, transparent);
  color: var(--tt-cyan);
}

.wb-inspector__selection-text {
  display: grid;
  min-width: 0;
}

.wb-inspector__selection-text strong {
  color: var(--text);
  font-size: 14px;
}

.wb-reset {
  position: relative;
  flex: none;
}

.wb-reset__btn {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}

.wb-reset__btn:hover:not(:disabled) {
  background: var(--panel);
  color: var(--text);
}

.wb-reset__btn:focus-visible {
  outline: 2px solid #00dce8;
  outline-offset: 1px;
}

.wb-panel {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 20px 18px 24px;
}

.wb-fields {
  border: 0;
  padding: 0;
  margin: 0;
  min-width: 0;
}

.wb-stack {
  display: grid;
  gap: 18px;
}

.wb-groups {
  display: grid;
  gap: 28px;
}

.wb-group + .wb-group {
  padding-top: 22px;
  border-top: 1px solid var(--line);
}

.wb-group__title {
  margin: 0 0 16px;
  color: var(--text);
  font-size: 12px;
  font-weight: 750;
}

.wb-row {
  display: grid;
  gap: 7px;
}

.wb-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-secondary);
}

.wb-color__inputs {
  display: flex;
  align-items: center;
  gap: 8px;
}

.wb-color input[type='color'] {
  width: 42px;
  height: 36px;
  padding: 2px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: transparent;
  cursor: pointer;
}

.wb-hex {
  flex: 1;
  min-width: 0;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel-solid);
  color: var(--text);
  font: inherit;
  font-size: 12px;
  padding: 9px 10px;
}

.wb-slider__control {
  display: flex;
  align-items: center;
  gap: 10px;
}

.wb-slider input[type='range'] {
  flex: 1;
  min-width: 0;
  accent-color: #00dce8;
}

.wb-slider output {
  flex: none;
  width: 52px;
  text-align: right;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  color: var(--text);
}

.wb-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.wb-switch {
  position: relative;
  width: 36px;
  height: 21px;
  flex: none;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--panel);
  cursor: pointer;
  padding: 0;
}

.wb-switch__thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 15px;
  height: 15px;
  border-radius: 50%;
  background: var(--text-muted);
  transition: transform 120ms ease-out, background-color 120ms ease-out;
}

.wb-switch.is-on {
  border-color: rgba(0, 220, 232, 0.6);
  background: rgba(0, 220, 232, 0.18);
}

.wb-switch.is-on .wb-switch__thumb {
  transform: translateX(15px);
  background: #00dce8;
}

.wb-switch:focus-visible {
  outline: 2px solid #00dce8;
  outline-offset: 2px;
}

.wb-align {
  gap: 8px;
}

.wb-segmented {
  display: flex;
  gap: 4px;
  padding: 3px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel-solid);
}

.wb-segmented button {
  flex: 1;
  padding: 6px 4px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--text-muted);
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.wb-segmented button.is-active {
  background: rgba(0, 220, 232, 0.14);
  color: var(--text);
}

.wb-segmented button:focus-visible {
  outline: 2px solid #00dce8;
  outline-offset: -2px;
}

.wb-text {
  position: relative;
  display: grid;
  gap: 7px;
}

.wb-label-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.wb-layer-actions {
  display: inline-flex;
  align-items: center;
  gap: 3px;
}

.wb-layer-editor,
.wb-layer-list,
.wb-layer__details {
  display: grid;
  gap: 10px;
}

.wb-layer-editor__hint {
  margin: 0;
  color: var(--text-muted);
  font-size: 11px;
  line-height: 1.45;
}

.wb-layer {
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--panel-solid);
}

.wb-layer.is-expanded {
  border-color: rgba(0, 220, 232, 0.45);
}

.wb-layer__header {
  display: flex;
  align-items: center;
  gap: 4px;
  min-height: 42px;
  padding: 5px 6px;
}

.wb-layer__select {
  display: flex;
  align-items: center;
  flex: 1;
  gap: 8px;
  min-width: 0;
  padding: 4px;
  border: 0;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
}

.wb-layer__number {
  display: grid;
  place-items: center;
  flex: none;
  width: 22px;
  height: 22px;
  border-radius: 6px;
  background: var(--panel);
  color: var(--text-muted);
  font-size: 10px;
}

.wb-layer__name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.wb-layer__header .wb-layer-actions button {
  display: grid;
  place-items: center;
  width: 22px;
  height: 24px;
  padding: 0;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--text-muted);
  font: inherit;
  font-size: 15px;
  cursor: pointer;
}

.wb-layer__header .wb-layer-actions button:hover:not(:disabled) {
  background: var(--panel);
  color: var(--text);
}

.wb-layer__header .wb-layer-actions button:disabled {
  opacity: 0.3;
  cursor: default;
}

.wb-layer__details {
  padding: 12px;
  border-top: 1px solid var(--line);
}

.wb-layer__input {
  width: 100%;
  min-width: 0;
  box-sizing: border-box;
  padding: 9px 10px;
  border: 1px solid var(--line);
  border-radius: 7px;
  outline: 0;
  background: var(--input-bg);
  color: var(--text);
  font: inherit;
  font-size: 12px;
  resize: vertical;
}

.wb-layer__input:focus-visible,
.wb-layer__select:focus-visible,
.wb-layer__header .wb-layer-actions button:focus-visible {
  outline: 2px solid #00dce8;
  outline-offset: 1px;
}

.wb-layer__tokens {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.wb-layer__tokens button {
  padding: 4px 6px;
  border: 1px solid var(--line);
  border-radius: 5px;
  background: var(--panel);
  color: var(--text-muted);
  font: inherit;
  font-size: 10px;
  cursor: pointer;
}

.wb-layer__tokens button:hover:not(:disabled) {
  border-color: rgba(0, 220, 232, 0.55);
  color: var(--text);
}

.wb-layer-action,
.wb-remove {
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--text-muted);
  font-size: 15px;
  line-height: 1;
  cursor: pointer;
}

.wb-layer-action:hover:not(:disabled),
.wb-remove:hover:not(:disabled) {
  background: var(--panel);
  color: var(--text);
}

.wb-layer-action:disabled {
  opacity: 0.35;
  cursor: default;
}

.wb-layer-action:focus-visible,
.wb-remove:focus-visible {
  outline: 2px solid #00dce8;
  outline-offset: 1px;
}

.wb-add {
  position: relative;
}

.wb-add__btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  width: 100%;
  padding: 8px;
  border: 1px dashed var(--line);
  border-radius: 8px;
  background: transparent;
  color: var(--text-muted);
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.wb-add__btn:hover:not(:disabled) {
  border-color: rgba(0, 220, 232, 0.55);
  color: #00dce8;
}

.wb-add__btn:focus-visible {
  outline: 2px solid #00dce8;
  outline-offset: 1px;
}

.wb-add .wb-menu {
  top: auto;
  bottom: calc(100% + 6px);
  min-width: 245px;
}

.wb-add__choice {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 6px 7px;
  color: var(--text-secondary);
  font-size: 11px;
}

.wb-add__choice + .wb-add__choice {
  border-top: 1px solid var(--line);
}

.wb-add__choice > span:last-child {
  display: flex;
  gap: 2px;
}

.wb-add .wb-menu .wb-add__choice button {
  width: auto;
  padding: 5px 7px;
  white-space: nowrap;
}

.wb-text__input {
  display: flex;
  gap: 6px;
}

.wb-text__input input {
  flex: 1;
  min-width: 0;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--input-bg);
  color: var(--text);
  font: inherit;
  font-size: 12px;
  padding: 10px 11px;
}

.wb-token-btn {
  flex: none;
  min-width: 38px;
  padding: 0 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--input-bg);
  color: var(--text-muted);
  font: inherit;
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
}

.wb-token-btn:hover:not(:disabled),
.wb-token-btn.is-open {
  border-color: rgba(0, 220, 232, 0.55);
  color: #00dce8;
}

.wb-token-btn:focus-visible {
  outline: 2px solid #00dce8;
  outline-offset: 1px;
}

.wb-menu {
  position: absolute;
  z-index: 5;
  top: calc(100% + 6px);
  right: 0;
  min-width: 190px;
  padding: 5px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--panel-solid);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.55);
}

.wb-menu button {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 10px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
  text-align: left;
}

.wb-menu button:hover {
  background: var(--panel);
}

.wb-menu button:focus-visible {
  outline: 2px solid #00dce8;
  outline-offset: -2px;
}

.wb-menu code {
  color: #00dce8;
  font-size: 11px;
}

.wb-menu--right {
  min-width: 170px;
}

.wb-scrim {
  position: absolute;
  inset: 0;
  z-index: 4;
}

.wb-error {
  margin: 0;
  padding: 10px 14px;
  border-top: 1px solid var(--line);
  color: #ff7a90;
  font-size: 12px;
}

.wb-color input:focus-visible,
.wb-hex:focus-visible,
.wb-text__input input:focus-visible,
.wb-slider input[type='range']:focus-visible {
  outline: 2px solid #00dce8;
  outline-offset: 1px;
}

@media (prefers-reduced-motion: reduce) {
  .wb-switch__thumb {
    transition: none;
  }
}

@media (max-width: 860px) {
  .widget-builder {
    grid-template-columns: 140px minmax(0, 1fr);
    grid-template-rows: minmax(0, 1.2fr) minmax(0, 1fr);
    overflow-y: auto;
  }

  .wb-rail {
    grid-row: 1 / -1;
  }

  .wb-inspector {
    border-left: 0;
    border-top: 1px solid var(--line);
    min-height: 260px;
  }

  .wb-canvas__frame {
    width: auto;
    height: 100%;
    max-width: 100%;
    max-height: none;
  }
}

@media (max-width: 640px) {
  .widget-builder {
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: auto minmax(280px, 44vh) minmax(0, 1fr);
  }

  .wb-rail {
    grid-row: auto;
    flex-direction: row;
    border-right: 0;
    border-bottom: 1px solid var(--line);
    overflow-x: auto;
    padding: 8px 10px;
  }

  .wb-rail__heading {
    display: none;
  }

  .wb-rail__item {
    flex: 1 0 auto;
    justify-content: center;
    min-height: 44px;
    padding: 8px 10px;
  }

  .wb-canvas__toolbar,
  .wb-canvas__footer {
    padding: 0 14px;
  }

  .wb-canvas__hint {
    display: none;
  }

  .wb-canvas__workarea {
    padding: 12px;
  }

}
</style>
