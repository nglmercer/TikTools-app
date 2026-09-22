<script lang="tsx">
import { ref, useId } from 'vue';
import { Modal } from './ui/Modal.vue';
import { Button } from './ui/Button.vue';
import {
  IconBolt,
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
import type { WidgetAlign, WidgetAvatarStyle, WidgetBadgeStyle, WidgetStyle, WidgetTemplateToken } from '../../widgets/sdk/template.ts';
import { textDefaults, type TextField } from '../../widgets/sdk/text.ts';
import { cloneDesign, EditorHistory } from './widgets-editor/history.ts';

type Props = {
  locale: Locale;
  kind: WidgetKind;
  title: string;
  design: WidgetStyle;
  onSave: (kind: WidgetKind, design: WidgetStyle) => Promise<void>;
  onClose: () => void;
};

type BuilderComponent = 'alert' | 'text' | 'image' | 'shape' | 'badge';
type InspectorTab = 'content' | 'style' | 'layout';

const COMPONENT_ORDER: BuilderComponent[] = ['alert', 'text', 'image', 'shape', 'badge'];
const TAB_ORDER: InspectorTab[] = ['content', 'style', 'layout'];

const COMPONENT_ICON = { alert: IconBolt, text: IconFormat, image: IconImage, shape: IconSquare, badge: IconStar } as const;
const COMPONENT_KEY = {
  alert: 'widgetsBuilderAlert', text: 'widgetsBuilderText', image: 'widgetsBuilderImage',
  shape: 'widgetsBuilderShape', badge: 'widgetsBuilderBadge',
} as const;
const TAB_KEY = { content: 'widgetsBuilderContent', style: 'widgetsBuilderStyle', layout: 'widgetsBuilderLayout' } as const;

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
  const component = ref<BuilderComponent>('alert');
  const tab = ref<InspectorTab>('content');
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

  const rails: BuilderComponent[] = props.kind === 'chat'
    ? COMPONENT_ORDER.filter((item) => item !== 'badge')
    : [...COMPONENT_ORDER];
  const defaultsText: Partial<Record<TextField, string>> = textDefaults[props.kind];
  const textFields = [...schema.textFields];
  const coreFields = [...schema.requiredTextFields];
  const tokens = [...schema.tokens];
  // The template schema is the source of truth for both screens. A saved
  // hiddenText list is the only reason a supported line is absent here.
  const visibleFields = ref<TextField[]>(textFields.filter((field) => !draft.value.hiddenText?.includes(field)));

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
    switch (component.value) {
      case 'alert':
        return ['background', 'textColor', 'accent', 'radius', 'borderColor', 'borderWidth', 'shadow', 'opacity', 'padding', 'gap', 'align', 'width', 'text'];
      case 'text':
        return ['text', 'textColor', 'accent'];
      case 'image':
        return ['avatar'];
      case 'shape':
        return ['background', 'borderColor', 'borderWidth', 'radius', 'shadow', 'opacity'];
      case 'badge':
        return textFields.includes('title') ? ['badge', 'text.title'] : ['badge'];
    }
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

  const addField = (field: TextField): void => {
    // Adding a field renders its line again with the default template.
    change('field:add', () => {
      visibleFields.value = [...visibleFields.value, field].sort(
        (left, right) => textFields.indexOf(left) - textFields.indexOf(right),
      );
      setHidden((draft.value.hiddenText ?? []).filter((item) => item !== field));
    });
    addMenu.value = false;
  };

  const resetSection = (): void => {
    const keys = resetSectionKeys();
    change('reset', () => {
      const next = { ...draft.value };
      for (const key of keys) {
        if (key === 'text.title') {
          if (next.text) {
            const text = { ...next.text };
            delete text.title;
            if (Object.keys(text).length > 0) next.text = text;
            else delete next.text;
          }
        } else {
          delete next[key as keyof WidgetStyle];
        }
      }
      draft.value = next;
      const resetTextFields = keys.includes('text')
        ? textFields
        : keys.includes('text.title')
          ? (['title'] as TextField[])
          : [];
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

  const moveTab = (current: InspectorTab, key: string): InspectorTab => {
    if (key === 'Home') return 'content';
    if (key === 'End') return 'layout';
    const index = TAB_ORDER.indexOf(current);
    const delta = key === 'ArrowRight' ? 1 : -1;
    return TAB_ORDER[(index + delta + TAB_ORDER.length) % TAB_ORDER.length] as InspectorTab;
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
    return (
      <div class="wb-text">
        <span class="wb-label-row">
          <label class="wb-label" for={inputId}>{label}</label>
          {removable ? (
            <button
              type="button"
              class="wb-remove"
              disabled={saving.value}
              aria-label={`${label}: ${t(props.locale, 'widgetsBuilderRemoveField')}`}
              onClick={() => removeField(field)}
            >
              ×
            </button>
          ) : null}
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
              <button type="button" role="menuitem" onClick={() => addField(field)}>
                <span>{t(props.locale, TEXT_LABEL_KEY[field])}</span>
              </button>
            ))}
          </div>
        ) : null}
      </div>
    );
  };

  const renderContent = () => {
    const selected = component.value;
    if (selected === 'alert' || selected === 'text') {
      return (
        <div class="wb-stack">
          {visibleFields.value.map((field) => renderTextField(field, !coreFields.includes(field)))}
          {renderAddField()}
        </div>
      );
    }
    if (selected === 'image') {
      return renderToggleRow(t(props.locale, 'widgetsBuilderVisible'), draft.value.avatar?.visible !== false, 'avatar.visible',
        (next) => updateAvatar('visible', next ? undefined : false));
    }
    if (selected === 'badge') {
      const hasTitle = textFields.includes('title');
      const titleVisible = hasTitle && visibleFields.value.includes('title');
      return (
        <div class="wb-stack">
          {titleVisible ? renderTextField('title', true) : null}
          {renderToggleRow(t(props.locale, 'widgetsBuilderVisible'), draft.value.badge?.visible !== false, 'badge.visible',
            (next) => updateBadge('visible', next ? undefined : false))}
          {hasTitle && !titleVisible ? (
            <button
              type="button"
              class="wb-add__btn"
              disabled={saving.value}
              onClick={() => addField('title')}
            >
              <IconPlus size={14} />
              <span>{t(props.locale, 'widgetsBuilderAddField')}</span>
            </button>
          ) : null}
        </div>
      );
    }
    return <p class="wb-empty">{t(props.locale, 'widgetsBuilderNoContent')}</p>;
  };

  const renderStyle = () => {
    const selected = component.value;
    const d = draft.value;
    if (selected === 'alert') {
      return (
        <div class="wb-stack">
          {renderColorRow(t(props.locale, 'widgetsDesignBackground'), d.background, defaults.background, 'color:background',
            (color) => { draft.value = { ...draft.value, background: color }; })}
          {renderColorRow(t(props.locale, 'widgetsDesignAccent'), d.accent, defaults.accent, 'color:accent',
            (color) => { draft.value = { ...draft.value, accent: color }; })}
          {renderColorRow(t(props.locale, 'widgetsDesignText'), d.textColor, defaults.textColor, 'color:textColor',
            (color) => { draft.value = { ...draft.value, textColor: color }; })}
          {renderSliderRow(t(props.locale, 'widgetsDesignRadius'), d.radius ?? defaults.radius, 0, 48, 1, ' px', 'radius',
            (next) => { draft.value = { ...draft.value, radius: next }; })}
          {renderColorRow(t(props.locale, 'widgetsBuilderBorder'), d.borderColor, defaults.borderColor, 'color:borderColor',
            (color) => { draft.value = { ...draft.value, borderColor: color }; })}
          {renderSliderRow(t(props.locale, 'widgetsBuilderBorderWidth'), d.borderWidth ?? defaults.borderWidth, 0, 8, 1, ' px', 'borderWidth',
            (next) => { draft.value = { ...draft.value, borderWidth: next }; })}
          {renderToggleRow(t(props.locale, 'widgetsBuilderShadow'), d.shadow !== false, 'shadow',
            (next) => {
              const nextDraft = { ...draft.value };
              if (next) delete nextDraft.shadow;
              else nextDraft.shadow = false;
              draft.value = nextDraft;
            })}
          {renderSliderRow(t(props.locale, 'widgetsBuilderOpacity'), d.opacity ?? defaults.opacity, 0, 100, 1, '%', 'opacity',
            (next) => { draft.value = { ...draft.value, opacity: next }; })}
        </div>
      );
    }
    if (selected === 'text') {
      return (
        <div class="wb-stack">
          {renderColorRow(t(props.locale, 'widgetsDesignText'), d.textColor, defaults.textColor, 'color:textColor',
            (color) => { draft.value = { ...draft.value, textColor: color }; })}
          {renderColorRow(t(props.locale, 'widgetsDesignAccent'), d.accent, defaults.accent, 'color:accent',
            (color) => { draft.value = { ...draft.value, accent: color }; })}
        </div>
      );
    }
    if (selected === 'image') {
      return (
        <div class="wb-stack">
          {renderSliderRow(t(props.locale, 'widgetsBuilderRound'), d.avatar?.radius ?? Math.min(80, (d.avatar?.size ?? defaults.avatarSize) / 2), 0, 80, 1, ' px', 'avatar.radius',
            (next) => updateAvatar('radius', next))}
          {renderColorRow(t(props.locale, 'widgetsBuilderBorder'), d.avatar?.borderColor, defaults.accent, 'avatar.borderColor',
            (color) => updateAvatar('borderColor', color))}
          {renderSliderRow(t(props.locale, 'widgetsBuilderBorderWidth'), d.avatar?.borderWidth ?? 0, 0, 8, 1, ' px', 'avatar.borderWidth',
            (next) => updateAvatar('borderWidth', next))}
        </div>
      );
    }
    if (selected === 'shape') {
      return (
        <div class="wb-stack">
          {renderColorRow(t(props.locale, 'widgetsDesignBackground'), d.background, defaults.background, 'color:background',
            (color) => { draft.value = { ...draft.value, background: color }; })}
          {renderColorRow(t(props.locale, 'widgetsBuilderBorder'), d.borderColor, defaults.borderColor, 'color:borderColor',
            (color) => { draft.value = { ...draft.value, borderColor: color }; })}
          {renderSliderRow(t(props.locale, 'widgetsBuilderBorderWidth'), d.borderWidth ?? defaults.borderWidth, 0, 8, 1, ' px', 'borderWidth',
            (next) => { draft.value = { ...draft.value, borderWidth: next }; })}
          {renderSliderRow(t(props.locale, 'widgetsDesignRadius'), d.radius ?? defaults.radius, 0, 48, 1, ' px', 'radius',
            (next) => { draft.value = { ...draft.value, radius: next }; })}
          {renderToggleRow(t(props.locale, 'widgetsBuilderShadow'), d.shadow !== false, 'shadow',
            (next) => {
              const nextDraft = { ...draft.value };
              if (next) delete nextDraft.shadow;
              else nextDraft.shadow = false;
              draft.value = nextDraft;
            })}
          {renderSliderRow(t(props.locale, 'widgetsBuilderOpacity'), d.opacity ?? defaults.opacity, 0, 100, 1, '%', 'opacity',
            (next) => { draft.value = { ...draft.value, opacity: next }; })}
        </div>
      );
    }
    return (
      <div class="wb-stack">
        {renderColorRow(t(props.locale, 'widgetsBuilderColor'), d.badge?.color, defaults.accent, 'badge.color',
          (color) => updateBadge('color', color))}
        {renderSliderRow(t(props.locale, 'widgetsBuilderFontSize'), d.badge?.fontSize ?? defaults.badgeSize, 8, 32, 1, ' px', 'badge.fontSize',
          (next) => updateBadge('fontSize', next))}
        {renderSliderRow(t(props.locale, 'widgetsBuilderWeight'), d.badge?.fontWeight ?? defaults.badgeWeight, 400, 900, 100, '', 'badge.fontWeight',
          (next) => updateBadge('fontWeight', next))}
        {renderSliderRow(t(props.locale, 'widgetsBuilderSpacing'), d.badge?.letterSpacing ?? defaults.badgeSpacing, 0, 8, 0.5, ' px', 'badge.letterSpacing',
          (next) => updateBadge('letterSpacing', next))}
      </div>
    );
  };

  const renderLayout = () => {
    const d = draft.value;
    const autoWidth = d.width === undefined;
    return (
      <div class="wb-stack">
        {component.value === 'image' ? renderSliderRow(t(props.locale, 'widgetsBuilderSize'), d.avatar?.size ?? defaults.avatarSize, 16, 160, 1, ' px', 'avatar.size',
          (next) => updateAvatar('size', next)) : null}
        {renderAlignRow()}
        {renderToggleRow(t(props.locale, 'widgetsBuilderAuto'), autoWidth, 'width.auto',
          (next) => {
            const nextDraft = { ...draft.value };
            if (next) delete nextDraft.width;
            else nextDraft.width = defaults.width;
            draft.value = nextDraft;
          })}
        {renderSliderRow(t(props.locale, 'widgetsBuilderWidth'), d.width ?? defaults.width, 240, 720, 4, ' px', 'width',
          (next) => { draft.value = { ...draft.value, width: next }; }, autoWidth)}
        {renderSliderRow(t(props.locale, 'widgetsBuilderPadding'), d.padding ?? defaults.padding, 0, 64, 1, ' px', 'padding',
          (next) => { draft.value = { ...draft.value, padding: next }; })}
        {renderSliderRow(t(props.locale, 'widgetsBuilderGap'), d.gap ?? defaults.gap, 0, 48, 1, ' px', 'gap',
          (next) => { draft.value = { ...draft.value, gap: next }; })}
      </div>
    );
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
          {rails.map((item) => {
            const Icon = COMPONENT_ICON[item];
            const selected = component.value === item;
            return (
              <button
                type="button"
                class={['wb-rail__item', selected ? 'is-active' : '']}
                aria-pressed={selected}
                onClick={() => { component.value = item; }}
              >
                <Icon size={20} />
                <span>{t(props.locale, COMPONENT_KEY[item])}</span>
              </button>
            );
          })}
        </nav>

        <section class="wb-canvas" aria-label={t(props.locale, 'widgetsPreview')}>
          <div class="wb-canvas__frame">
            <WidgetHost template={template} mode="preview" design={draft.value} replayKey={replay.value} />
            <Button
              variant="soft"
              size="sm"
              icon={<IconPlay size={14} />}
              iconOnly
              tooltip={t(props.locale, 'widgetsReplay')}
              onClick={() => { replay.value++; }}
            />
          </div>
        </section>

        <aside class="wb-inspector">
          <div class="wb-inspector__head">
            <div class="wb-tabs" role="tablist" aria-label={t(props.locale, 'widgetsBuilder')}>
              {TAB_ORDER.map((name) => (
                <button
                  type="button"
                  role="tab"
                  id={`${baseId}-tab-${name}`}
                  aria-controls={`${baseId}-panel`}
                  aria-selected={tab.value === name}
                  tabindex={tab.value === name ? 0 : -1}
                  class={tab.value === name ? 'is-active' : ''}
                  onClick={() => { tab.value = name; }}
                  onKeydown={(event) => {
                    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
                    event.preventDefault();
                    const next = moveTab(name, event.key);
                    tab.value = next;
                    document.getElementById(`${baseId}-tab-${next}`)?.focus();
                  }}
                >
                  {t(props.locale, TAB_KEY[name])}
                </button>
              ))}
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
            role="tabpanel"
            id={`${baseId}-panel`}
            aria-labelledby={`${baseId}-tab-${tab.value}`}
          >
            <fieldset class="wb-fields" disabled={saving.value}>
              {tab.value === 'content' ? renderContent() : tab.value === 'style' ? renderStyle() : renderLayout()}
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
  grid-template-columns: 92px minmax(0, 1fr) 300px;
  gap: 0;
  height: 100%;
  min-height: 0;
  font-size: 13px;
}

.wb-rail {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 14px 10px;
  border-right: 1px solid var(--line);
  overflow-y: auto;
}

.wb-rail__item {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 5px;
  min-height: 60px;
  padding: 8px 4px;
  border: 1px solid transparent;
  border-radius: 9px;
  background: transparent;
  color: var(--text-muted);
  font: inherit;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
}

.wb-rail__item:hover {
  background: var(--panel);
  color: var(--text);
}

.wb-rail__item.is-active {
  border-color: rgba(0, 220, 232, 0.55);
  background: rgba(0, 220, 232, 0.08);
  color: var(--text);
}

.wb-rail__item:focus-visible {
  outline: 2px solid #00dce8;
  outline-offset: 1px;
}

.wb-canvas {
  min-width: 0;
  min-height: 0;
  padding: 14px;
  display: flex;
}

.wb-canvas__frame {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 10px;
  background:
    repeating-conic-gradient(rgba(255, 255, 255, 0.04) 0% 25%, transparent 0% 50%) 0 0 / 22px 22px,
    #101014;
}

.wb-canvas__frame > .ui-btn {
  position: absolute;
  left: 10px;
  bottom: 10px;
}

.wb-inspector {
  display: flex;
  flex-direction: column;
  min-height: 0;
  border-left: 1px solid var(--line);
}

.wb-inspector__head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--line);
}

.wb-tabs {
  display: flex;
  flex: 1;
  gap: 2px;
  min-width: 0;
}

.wb-tabs button {
  flex: 1;
  padding: 8px 4px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--text-muted);
  font: inherit;
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
}

.wb-tabs button:hover {
  color: var(--text);
  background: var(--panel);
}

.wb-tabs button.is-active {
  color: var(--text);
  background: rgba(0, 220, 232, 0.1);
}

.wb-tabs button:focus-visible {
  outline: 2px solid #00dce8;
  outline-offset: -2px;
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
  padding: 14px 14px 18px;
}

.wb-fields {
  border: 0;
  padding: 0;
  margin: 0;
  min-width: 0;
}

.wb-stack {
  display: grid;
  gap: 13px;
}

.wb-empty {
  margin: 4px 0;
  color: var(--text-muted);
  font-size: 12px;
}

.wb-row {
  display: grid;
  gap: 7px;
}

.wb-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
}

.wb-color__inputs {
  display: flex;
  align-items: center;
  gap: 8px;
}

.wb-color input[type='color'] {
  width: 40px;
  height: 30px;
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
  border-radius: 7px;
  background: var(--panel-solid);
  color: var(--text);
  font: inherit;
  font-size: 12px;
  padding: 7px 9px;
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

.wb-remove:hover:not(:disabled) {
  background: var(--panel);
  color: var(--text);
}

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
}

.wb-text__input {
  display: flex;
  gap: 6px;
}

.wb-text__input input {
  flex: 1;
  min-width: 0;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--panel-solid);
  color: var(--text);
  font: inherit;
  font-size: 12px;
  padding: 7px 9px;
}

.wb-token-btn {
  flex: none;
  padding: 0 9px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: transparent;
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

@media (max-width: 980px) {
  .widget-builder {
    grid-template-columns: 76px minmax(0, 1fr);
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
    padding: 10px;
  }

  .wb-rail__item {
    flex: 1 0 64px;
    min-height: 54px;
  }
}
</style>
