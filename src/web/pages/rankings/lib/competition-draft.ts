import type { CompetitionDefinition, CompetitionOption } from '@/entities/competition/types.ts';

export type InputRow = { key: string; value: string; kind: 'string' | 'number' | 'boolean' };
/** Editable text form of an option; converted back with `toOption`. */
export interface OptionDraft {
  id: string; label: string; command: string; actionId: string; imageUrl: string; giftIds: string;
  originalAction: CompetitionOption['onWin']; inputs: InputRow[];
}
export type CompetitionType = Exclude<CompetitionDefinition['kind'], 'weighted_poll'>;
export const competitionTypes: CompetitionType[] = ['poll', 'team_battle', 'roulette', 'random_draw', 'weighted_draw', 'leaderboard_challenge'];

export const blankOption = (): OptionDraft => ({ id: '', label: '', command: '', actionId: '', imageUrl: '', giftIds: '', originalAction: null, inputs: [] });

export function draftFromOption(option: CompetitionOption): OptionDraft {
  return {
    id: option.id, label: option.label, command: option.commands.join(', '), actionId: option.onWin?.actionId ?? '', imageUrl: option.imageUrl ?? '', giftIds: option.giftIds.join(', '), originalAction: option.onWin,
    inputs: Object.entries(option.onWin?.inputs ?? {}).flatMap(([key, value]): InputRow[] => typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean' ? [{ key, value: String(value), kind: typeof value as InputRow['kind'] }] : []),
  };
}

export const splitList = (value: string): string[] => value.split(',').map((item) => item.trim()).filter(Boolean);

export function invalidInputs(draft: OptionDraft): boolean {
  const keys = draft.inputs.map((input) => input.key.trim());
  return keys.some((key) => !key || key.length > 128) || new Set(keys).size !== keys.length
    || draft.inputs.some((input) => (input.kind === 'number' && (input.value.trim() === '' || !Number.isFinite(Number(input.value)))) || (input.kind === 'boolean' && !['true', 'false'].includes(input.value)));
}

/** Non-scalar inputs of an unchanged action are preserved; scalar rows replace the rest. */
export function toOption(draft: OptionDraft): CompetitionOption {
  const actionId = draft.actionId.trim();
  const inputs: Record<string, unknown> = Object.create(null);
  if (draft.originalAction?.actionId === actionId) for (const [key, value] of Object.entries(draft.originalAction.inputs)) if (value === null || typeof value === 'object') inputs[key] = value;
  for (const input of draft.inputs) inputs[input.key.trim()] = input.kind === 'number' ? Number(input.value) : input.kind === 'boolean' ? input.value === 'true' : input.value;
  return { id: draft.id.trim(), label: draft.label.trim(), commands: splitList(draft.command), giftIds: splitList(draft.giftIds), imageUrl: draft.imageUrl.trim() || null, onWin: actionId ? { actionId, inputs } : null };
}
