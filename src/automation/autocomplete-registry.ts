/**
 * Global autocomplete ownership registry.
 *
 * The generated event registry is static: it documents every `event.intel.*`
 * path the contract allows, even while the processor plugin that emits those
 * paths is disabled or uninstalled. Suggesting them anyway sends users down a
 * dead end (filters that can never match, templates that resolve to nothing).
 *
 * This module is the single choke point that keeps suggestions honest. It
 * answers one question — "may this path be suggested right now?" — from two
 * inputs:
 *
 * 1. Ownership. `event.intel.providers.<pluginId>.*` is attributed by
 *    convention to `<pluginId>` (the host merge guarantees that namespace, see
 *    `tiktools-core::plugin_processors::merge`). Shared stable views such as
 *    `event.intel.comment.*` cannot be attributed by naming, so the plugin
 *    that promotes them registers an explicit contribution instead. Any
 *    processor that promotes the same stable keys registers the same
 *    prefixes; a shared path stays visible while at least one owner is
 *    available.
 * 2. Availability. The web layer syncs installed/enabled states from every
 *    behavior snapshot (`syncAutocompletePluginStates`). Unknown plugins fail
 *    open (suggested): old hosts without a snapshot and unit tests without a
 *    sync keep today's behavior.
 *
 * Public API for plugins and host integrations:
 *
 * - `registerAutocompleteContribution` pushes ownership and/or extra
 *   suggestion fields (paths the static registry cannot know, such as a
 *   provider-namespaced verdict); re-registering an id replaces it.
 * - `unregisterAutocompleteContribution` /
 *   `unregisterAutocompleteContributionsForPlugin` undo a contribution
 *   (hot-unload, tests). Enable/disable needs no (un)registration: the
 *   snapshot sync gates every registered contribution automatically.
 *
 * UI-agnostic on purpose: `src/automation` never imports from `src/web`.
 * Contribution fields feed template autocomplete only — the event registry
 * keeps its sample-drift guarantee (every registry path resolves against its
 * sample event), so provider-namespaced paths that samples cannot carry must
 * never be appended there.
 */

export type AutocompleteScalarKind = 'string' | 'number' | 'boolean';

/** One extra suggestion a plugin pushes (unknown to the static registry). */
export interface AutocompleteContributionField {
  /** Dotted path, exactly what filters and templates store. */
  path: string;
  kind: AutocompleteScalarKind;
  label: { en: string; es: string };
  hint?: { en: string; es: string };
}

export interface AutocompleteContribution {
  /** Stable id; re-registering the same id replaces the contribution. */
  id: string;
  /** Owning plugin, e.g. `textintel`. Gated by its installed/enabled state. */
  pluginId: string;
  /** Owned namespaces, e.g. `event.intel.comment.` (trailing dot optional). */
  prefixes?: string[];
  /** Owned exact paths (gated, but not added as suggestions). */
  paths?: string[];
  /** Extra suggestion items, gated by the owner like everything else. */
  fields?: AutocompleteContributionField[];
  /** Event types this contribution applies to; absent means every trigger. */
  triggers?: string[];
}

export interface AutocompletePluginState {
  installed: boolean;
  enabled: boolean;
}

interface StoredContribution {
  id: string;
  pluginId: string;
  prefixes: string[];
  paths: string[];
  fields: AutocompleteContributionField[];
  triggers: string[] | undefined;
}

const contributions = new Map<string, StoredContribution>();
const pluginStates = new Map<string, AutocompletePluginState>();

const PROVIDER_PATH = /^event\.intel\.providers\.([^.]+)(?:\.|$)/;

function cleanList(values: string[] | undefined): string[] {
  return [...new Set((values ?? []).map((value) => value.trim()).filter(Boolean))];
}

/** `event.intel.providers.<pluginId>...` → `<pluginId>`; anything else → undefined. */
export function providerPluginIdForPath(path: string): string | undefined {
  return PROVIDER_PATH.exec(path.trim())?.[1];
}

function prefixMatches(path: string, prefix: string): boolean {
  const clean = prefix.endsWith('.') ? prefix.slice(0, -1) : prefix;
  if (!clean) return false;
  return path === clean || path.startsWith(`${clean}.`);
}

function triggerMatches(contribution: StoredContribution, trigger: string | undefined): boolean {
  // A union query (no trigger) spans every trigger a scoped contribution
  // could apply to, so scoped contributions stay in force there.
  if (trigger === undefined) return true;
  if (!contribution.triggers) return true;
  return contribution.triggers.includes(trigger);
}

function contributionOwnsPath(contribution: StoredContribution, path: string): boolean {
  const candidate = path.trim();
  if (!candidate) return false;
  return contribution.prefixes.some((prefix) => prefixMatches(candidate, prefix))
    || contribution.paths.includes(candidate)
    || contribution.fields.some((field) => field.path === candidate);
}

/**
 * Push (or replace) a plugin's autocomplete contribution. Throws on an empty
 * id or plugin id, mirroring the other registry helpers.
 */
export function registerAutocompleteContribution(contribution: AutocompleteContribution): void {
  const id = contribution.id.trim();
  const pluginId = contribution.pluginId.trim();
  if (!id || !pluginId) throw new Error('An autocomplete contribution needs an id and a pluginId.');
  const fields = (contribution.fields ?? [])
    .filter((field) => field && typeof field.path === 'string' && field.path.trim() !== '')
    .map((field) => ({
      path: field.path.trim(),
      kind: field.kind,
      label: { en: field.label.en, es: field.label.es },
      hint: field.hint ? { en: field.hint.en, es: field.hint.es } : undefined,
    }));
  contributions.set(id, {
    id,
    pluginId,
    prefixes: cleanList(contribution.prefixes),
    paths: cleanList(contribution.paths),
    fields,
    triggers: cleanList(contribution.triggers).length > 0 ? cleanList(contribution.triggers) : undefined,
  });
}

/** Undo one contribution by id. Returns false when nothing was registered. */
export function unregisterAutocompleteContribution(id: string): boolean {
  return contributions.delete(id.trim());
}

/** Undo every contribution owned by one plugin. Returns the removed ids. */
export function unregisterAutocompleteContributionsForPlugin(pluginId: string): string[] {
  const owner = pluginId.trim();
  const removed: string[] = [];
  for (const [id, contribution] of contributions) {
    if (contribution.pluginId === owner) {
      contributions.delete(id);
      removed.push(id);
    }
  }
  return removed;
}

/** Snapshot of the registered contributions (copies; mutating them is safe). */
export function autocompleteContributions(): AutocompleteContribution[] {
  return [...contributions.values()].map((contribution) => ({
    id: contribution.id,
    pluginId: contribution.pluginId,
    prefixes: [...contribution.prefixes],
    paths: [...contribution.paths],
    fields: contribution.fields.map((field) => ({ ...field, label: { ...field.label }, hint: field.hint ? { ...field.hint } : undefined })),
    triggers: contribution.triggers ? [...contribution.triggers] : undefined,
  }));
}

/** Record one plugin's installed/enabled state. Empty ids are ignored. */
export function setAutocompletePluginState(pluginId: string, state: AutocompletePluginState): void {
  const id = pluginId.trim();
  if (!id) return;
  pluginStates.set(id, { installed: state.installed, enabled: state.enabled });
}

/**
 * Replace every plugin state from the behavior snapshot. The snapshot is
 * authoritative: plugins absent from it return to fail-open.
 */
export function syncAutocompletePluginStates(
  plugins: Array<{ id: string; installed: boolean; enabled: boolean }>,
): void {
  pluginStates.clear();
  for (const plugin of plugins) {
    setAutocompletePluginState(plugin.id, { installed: plugin.installed, enabled: plugin.enabled });
  }
}

/** True when the plugin may currently emit events. Unknown plugins fail open. */
export function isAutocompletePluginAvailable(pluginId: string): boolean {
  const state = pluginStates.get(pluginId.trim());
  if (!state) return true;
  return state.installed && state.enabled;
}

/**
 * True when a path may be suggested for a trigger (or for a union query when
 * the trigger is undefined). Core paths are always available; owned paths
 * need at least one available owner.
 */
export function isAutocompletePathAvailable(path: string, trigger?: string): boolean {
  const candidate = path.trim();
  if (!candidate) return false;
  const providerId = providerPluginIdForPath(candidate);
  if (providerId !== undefined) return isAutocompletePluginAvailable(providerId);
  let owned = false;
  for (const contribution of contributions.values()) {
    if (!triggerMatches(contribution, trigger) || !contributionOwnsPath(contribution, candidate)) continue;
    owned = true;
    if (isAutocompletePluginAvailable(contribution.pluginId)) return true;
  }
  return !owned;
}

/**
 * Extra suggestion fields visible for a trigger right now (union when the
 * trigger is undefined). Skips contributions whose owner is unavailable
 * unless `includeDisabled` is set. Feeds template autocomplete only — see
 * the module docstring for why these never enter the event registry.
 */
export function contributionFieldsForTrigger(
  trigger?: string,
  options?: { includeDisabled?: boolean },
): AutocompleteContributionField[] {
  const includeDisabled = options?.includeDisabled === true;
  const out: AutocompleteContributionField[] = [];
  for (const contribution of contributions.values()) {
    if (!triggerMatches(contribution, trigger)) continue;
    if (!includeDisabled && !isAutocompletePluginAvailable(contribution.pluginId)) continue;
    for (const field of contribution.fields) {
      if (out.some((entry) => entry.path === field.path)) continue;
      out.push(field);
    }
  }
  return out;
}

function registerBuiltins(): void {
  // Stable views the TextIntel processor promotes (`comment`/`user` plus
  // their TTS projections). Any future processor promoting the same stable
  // keys registers the same prefixes; the OR rule in
  // `isAutocompletePathAvailable` keeps them visible while at least one
  // promoter is available. `event.intel.processing.status` is host-stamped
  // (core), so it stays ungated.
  registerAutocompleteContribution({
    id: 'textintel.stable-views',
    pluginId: 'textintel',
    prefixes: ['event.intel.comment.', 'event.intel.user.'],
  });
  // Provider-namespaced verdict the static registry deliberately skips
  // (`providers` is untyped JSON). Offered for the chat trigger TextIntel
  // enriches, and for union queries; hidden while TextIntel is unavailable.
  registerAutocompleteContribution({
    id: 'textintel.moderation',
    pluginId: 'textintel',
    fields: [
      {
        path: 'event.intel.providers.textintel.comment.moderation.blocked',
        kind: 'boolean',
        label: { en: 'Moderation blocked', es: 'Bloqueado por moderación' },
        hint: {
          en: 'True when TextIntel blocked this chat message (spam or bad words).',
          es: 'Verdadero cuando TextIntel bloqueó este mensaje del chat (spam o palabras prohibidas).',
        },
      },
    ],
    triggers: ['tiktok.chat'],
  });
}

registerBuiltins();

/**
 * Test helper: drops every plugin state and custom contribution, then
 * restores the builtins. Always call it (or the try/finally equivalent)
 * around tests that sync states or register contributions — the registry is
 * process-global and leaks between tests otherwise.
 */
export function resetAutocompleteRegistry(): void {
  contributions.clear();
  pluginStates.clear();
  registerBuiltins();
}
