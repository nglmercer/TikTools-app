/**
 * Package-local option-source helpers (moved from the main frontend).
 *
 * Only manifest-declared `plugin-action-options:<action>:<field>` sources
 * are ever parsed: the plugin UI cannot steer the host at arbitrary URLs.
 * Mirrors the host parser; the two are pinned by identical fixtures.
 */

import type { ActionOptionItem } from '../types.ts';

const OPTION_SOURCE_PREFIX = 'plugin-action-options:';

export function parseOptionSourceId(
  source: string,
): { actionType: string; field: string } | undefined {
  if (!source.startsWith(OPTION_SOURCE_PREFIX)) return undefined;
  const rest = source.slice(OPTION_SOURCE_PREFIX.length);
  const separator = rest.indexOf(':');
  if (separator <= 0 || separator !== rest.lastIndexOf(':')) return undefined;
  const actionType = rest.slice(0, separator);
  const field = rest.slice(separator + 1);
  if (!actionType || !field) return undefined;
  return { actionType, field };
}

/** Where an outputs select sends its choice: action type plus config field. */
export type OutputsTarget = { actionType: string; field: string };

/**
 * Derives the immediate-execution target from an outputs option source id.
 * The source already names `<actionType>:<field>`, so one manifest marker
 * feeds the selector options and addresses the switch action.
 */
export function outputsTarget(source: string): OutputsTarget | undefined {
  const parsed = parseOptionSourceId(source);
  if (!parsed) return undefined;
  return { actionType: parsed.actionType, field: parsed.field };
}

/**
 * True when an option-source error means "endpoint not found". Older servers
 * predate the audio API and answer 404; anything else is a generic outage.
 * The `HTTP 404` prefix contract is pinned host-side by
 * `missing_option_endpoint_reports_its_status`.
 */
export function isNotFoundOptionError(error: string | undefined): boolean {
  if (!error) return false;
  return /^HTTP 404(?:\s|$)/.test(error.trim());
}

/** Select rows: server options plus the live value when it is not listed. */
export function outputOptions(
  outputs: ActionOptionItem[],
  current: string,
): Array<{ value: string; label: string }> {
  const options = outputs.map((output) => ({ value: output.value, label: output.label || output.value }));
  if (current && !options.some((option) => option.value === current)) {
    options.unshift({ value: current, label: current });
  }
  return options;
}

export type OutputsCardState =
  | { kind: 'hidden' }
  | { kind: 'loading' }
  | { kind: 'unavailable'; unsupported: boolean }
  | { kind: 'empty' }
  | { kind: 'ready' };

/**
 * Display state for the TTS audio-output card. Server fetch errors hide the
 * selector (never the panel); an empty list is a distinct refreshable note.
 */
export function outputsCardState(args: {
  supported: boolean;
  outputs: ActionOptionItem[] | undefined;
  outputsError: string | undefined;
}): OutputsCardState {
  if (!args.supported) return { kind: 'hidden' };
  if (args.outputsError) {
    return { kind: 'unavailable', unsupported: isNotFoundOptionError(args.outputsError) };
  }
  if (!args.outputs) return { kind: 'loading' };
  if (args.outputs.length === 0) return { kind: 'empty' };
  return { kind: 'ready' };
}
