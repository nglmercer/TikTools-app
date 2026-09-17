import { ref, watch, type Ref } from 'vue';
import {
  createAutocompleteController,
  isExplicitInvokeKey,
  resolveAutocompleteKey,
  type AutocompleteController,
  type AutocompleteControllerOptions,
  type AutocompleteControllerSnapshot,
  type AutocompleteLabels,
  type InsertResult,
  type PresetItem,
} from './autocomplete-controller.ts';
import type { AutocompleteMode, SuggestionItem, SuggestionRow, SuggestionScope } from './types.ts';
import type { Locale } from '../../i18n.ts';

/* ------------------------------------------------------------------ */
/* Fallback controller: two Phase 2 controllers behind one interface.  */
/* Pure (no Vue) so bun tests cover the routing directly.              */
/* ------------------------------------------------------------------ */

/**
 * Routes one input through two controllers: the primary while it is open,
 * otherwise the fallback. Used for template inputs that also offer URL
 * presets — inside `{{ }}` the template controller owns the dropdown,
 * outside it the preset controller does. The two never mix in one list.
 *
 * Explicit invoke (Ctrl+Space) always targets the primary; dismiss closes
 * both. Navigation/commit/hover follow whichever is currently active.
 */
export function createFallbackAutocompleteController(
  primary: AutocompleteController,
  fallback: AutocompleteController,
): AutocompleteController {
  let primarySnapshot: AutocompleteControllerSnapshot | null = null;

  const active = (): AutocompleteController =>
    primarySnapshot?.open === true ? primary : fallback;

  const refresh = (
    primaryNext: AutocompleteControllerSnapshot,
    fallbackNext: AutocompleteControllerSnapshot,
  ): AutocompleteControllerSnapshot => {
    primarySnapshot = primaryNext;
    return primaryNext.open ? primaryNext : fallbackNext;
  };

  return {
    snapshot: () => refresh(primary.snapshot(), fallback.snapshot()),
    update: (input) => refresh(primary.update(input), fallback.update(input)),
    invoke: () => {
      const next = primary.invoke();
      // Recompute the fallback against the same state so `active()` and the
      // returned snapshot agree even when the primary stays closed.
      const cold = fallback.snapshot();
      return refresh(next, cold);
    },
    dismiss: () => {
      primary.dismiss();
      const next = fallback.dismiss();
      primarySnapshot = primary.snapshot();
      return primarySnapshot.open ? primarySnapshot : next;
    },
    hover: (globalIndex) => {
      const controller = active();
      const next = controller.hover(globalIndex);
      if (controller === primary) primarySnapshot = next;
      return controller === primary ? next : refresh(primarySnapshot ?? primary.snapshot(), next);
    },
    key: (keyValue) => active().key(keyValue),
    pendingRow: () => active().pendingRow(),
    commit: (value, caret) => {
      const result = active().commit(value, caret);
      primarySnapshot = primary.snapshot();
      return result;
    },
  };
}

/* ------------------------------------------------------------------ */
/* Vue glue: controller lifecycle for TemplateField / URL / CodeEditor.*/
/* Every dropdown in the app renders through this + AutocompletePopover*/
/* + AutocompleteList — no per-input filter/index/keyboard/insert code.*/
/* ------------------------------------------------------------------ */

export type AutocompleteInputOptions = {
  mode: AutocompleteMode;
  /** Template pool (event variables). Never consulted in preset mode. */
  suggestions?: readonly SuggestionItem[];
  /**
   * URL presets. Template mode only: shown outside `{{ }}` via a second
   * preset controller (see `createFallbackAutocompleteController`).
   */
  presets?: readonly PresetItem[];
  /** Options pool (dynamic lists). Only consulted in options mode. */
  options?: readonly SuggestionItem[];
  /** Consumer scope for template filtering (default `generic`). */
  scope?: SuggestionScope;
  /** Template gate: only template-capable inputs pass true. */
  supportsTemplates?: boolean;
  locale?: Locale;
  labels?: AutocompleteLabels;
  limit?: number;
  presetLimit?: number;
  /** Open the options list on focus-empty (default true). */
  openOptionsOnFocus?: boolean;
};

export type AutocompleteInput = {
  /** Latest snapshot (open/sections/rows/active index). */
  snapshot: Ref<AutocompleteControllerSnapshot>;
  /** Stable listbox id for `aria-controls`/`aria-activedescendant`. */
  listId: string;
  /** Push the input state after every value/caret/focus change. */
  update: (value: string, caret: number, focused: boolean) => void;
  /**
   * Handle one key. Returns `commit` when the caller should apply
   * `commit(value, caret)`, `dismissed` on Escape, else null (including
   * navigation, which only moves the active index). Handled keys are
   * `preventDefault`-ed; with a closed dropdown nothing is handled so
   * Enter/Tab/Escape keep their native behavior.
   */
  keydown: (event: KeyboardEvent) => 'commit' | 'dismissed' | null;
  /** Hover sync: point the active index at the hovered row. */
  hover: (globalIndex: number) => void;
  /** Commit the row under the pointer (hover-then-commit by stable key). */
  pickRow: (value: string, caret: number, key: string) => (InsertResult & { row: SuggestionRow }) | null;
  /** Insert the pending row (mode-correct replacement) and close. */
  commit: (value: string, caret: number) => (InsertResult & { row: SuggestionRow }) | null;
  /** Ctrl+Space: open from anywhere. */
  invoke: () => void;
  /** Escape: close, keep focus on the input (never blurs). */
  dismiss: () => void;
};

let autocompleteInputCounter = 0;

function toControllerOptions(options: AutocompleteInputOptions): AutocompleteControllerOptions {
  return {
    mode: options.mode,
    suggestions: options.suggestions,
    presets: options.mode === 'template' ? undefined : options.presets,
    options: options.options,
    scope: options.scope,
    supportsTemplates: options.supportsTemplates,
    locale: options.locale,
    labels: options.labels,
    limit: options.limit,
    presetLimit: options.presetLimit,
    openOptionsOnFocus: options.openOptionsOnFocus,
  };
}

function toPresetControllerOptions(options: AutocompleteInputOptions): AutocompleteControllerOptions {
  return {
    mode: 'preset',
    presets: options.presets,
    locale: options.locale,
    labels: options.labels,
    presetLimit: options.limit ?? options.presetLimit,
  };
}

function hasDualPresets(options: AutocompleteInputOptions): boolean {
  return options.mode === 'template' && (options.presets?.length ?? 0) > 0;
}

/**
 * Binds Phase 2 controller(s) to one text input. `resolve` reads the live
 * pools from the caller's props on every state push, so per-render array
 * identities never reset navigation or the after-commit latch; scalar
 * options (mode/scope/locale/…) recreate the controllers when they change.
 */
export function useAutocompleteInput(resolve: () => AutocompleteInputOptions): AutocompleteInput {
  const listId = `tt-ac-${(autocompleteInputCounter += 1)}`;
  // Mutable holder: the controllers read pools live on every `compute()`,
  // so assigning here keeps one controller instance across renders.
  const live = toControllerOptions(resolve());
  const livePreset = toPresetControllerOptions(resolve());
  let controller = buildController(resolve(), live, livePreset);
  const snapshot = ref<AutocompleteControllerSnapshot>(controller.snapshot());

  function buildController(
    current: AutocompleteInputOptions,
    main: AutocompleteControllerOptions,
    preset: AutocompleteControllerOptions,
  ): AutocompleteController {
    const mainController = createAutocompleteController(main);
    if (!hasDualPresets(current)) return mainController;
    return createFallbackAutocompleteController(mainController, createAutocompleteController(preset));
  }

  function syncPools(current: AutocompleteInputOptions): void {
    live.suggestions = current.suggestions;
    live.presets = current.mode === 'template' ? undefined : current.presets;
    live.options = current.options;
    livePreset.presets = current.presets;
  }

  // Scalars are construction-time in the controller; pools stay live.
  watch(
    () => {
      const current = resolve();
      return [
        current.mode,
        current.scope ?? 'generic',
        current.supportsTemplates === true,
        current.locale ?? 'en',
        current.labels ?? null,
        current.limit ?? 12,
        current.presetLimit ?? 8,
        current.openOptionsOnFocus !== false,
        hasDualPresets(current),
      ] as const;
    },
    () => {
      const current = resolve();
      syncPools(current);
      Object.assign(live, toControllerOptions(current));
      Object.assign(livePreset, toPresetControllerOptions(current));
      controller = buildController(current, live, livePreset);
      snapshot.value = controller.snapshot();
    },
  );

  const refresh = (next: AutocompleteControllerSnapshot): AutocompleteControllerSnapshot => {
    snapshot.value = next;
    return next;
  };

  return {
    snapshot,
    listId,
    update: (value, caret, focused) => {
      syncPools(resolve());
      refresh(controller.update({ value, caret, focused }));
    },
    keydown: (event) => {
      if (isExplicitInvokeKey(event)) {
        event.preventDefault();
        syncPools(resolve());
        refresh(controller.invoke());
        return null;
      }
      if (resolveAutocompleteKey(event.key) === null) return null;
      if (!snapshot.value.open) return null;
      event.preventDefault();
      const result = controller.key(event.key);
      refresh(controller.snapshot());
      return result;
    },
    hover: (globalIndex) => {
      refresh(controller.hover(globalIndex));
    },
    pickRow: (value, caret, key) => {
      const rows = snapshot.value.sections.flatMap((section) => section.rows);
      const index = rows.findIndex((row) => row.key === key || row.item.value === key);
      if (index < 0) return null;
      refresh(controller.hover(index));
      const result = controller.commit(value, caret);
      refresh(controller.snapshot());
      return result;
    },
    commit: (value, caret) => {
      const result = controller.commit(value, caret);
      refresh(controller.snapshot());
      return result;
    },
    invoke: () => {
      syncPools(resolve());
      refresh(controller.invoke());
    },
    dismiss: () => {
      refresh(controller.dismiss());
    },
  };
}
