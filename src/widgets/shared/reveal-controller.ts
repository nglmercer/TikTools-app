/** Presentation-only adapter for committed Rust results. It never draws. */
import { isRecord } from './event-parser.ts';
export interface RevealResult { competitionId: string; rankingId: string; generation: number; winner: string | null; mode: 'highest_score' | 'uniform_random' | 'weighted_random'; eligible: { key: string; weight: number }[]; eligibleCount: number; otherWeight: number }
export function parseRevealResult(envelope: unknown): RevealResult | null {
  if (!isRecord(envelope) || envelope['topic'] !== 'competition.result' || !isRecord(envelope['data'])) return null;
  const result = envelope['data']['result'];
  if (!isRecord(result) || result['schemaVersion'] !== 1 || typeof result['competitionId'] !== 'string' || !result['competitionId'] || typeof result['rankingId'] !== 'string' || !result['rankingId'] || !Number.isSafeInteger(result['generation']) || (result['generation'] as number) < 1 || !isRecord(result['selection'])) return null;
  const selection = result['selection'];
  if (!['highest_score', 'uniform_random', 'weighted_random'].includes(String(selection['mode'])) || !Array.isArray(selection['eligible']) || selection['eligible'].length === 0 || selection['eligible'].length > 256 || (selection['winner'] !== null && typeof selection['winner'] !== 'string')) return null;
  const eligible: RevealResult['eligible'] = [];
  const keys = new Set<string>();
  for (const entry of selection['eligible']) {
    if (!isRecord(entry) || typeof entry['key'] !== 'string' || !entry['key'] || entry['key'].length > 512 || keys.has(entry['key']) || !Number.isSafeInteger(entry['weight']) || (entry['weight'] as number) < 0) return null;
    keys.add(entry['key']); eligible.push({ key: entry['key'], weight: entry['weight'] as number });
  }
  if (selection['winner'] !== null && !keys.has(selection['winner'] as string)) return null;
  const eligibleCount = result['eligibleCount'] ?? eligible.length;
  const otherWeight = result['otherWeight'] ?? 0;
  if (!Number.isSafeInteger(eligibleCount) || (eligibleCount as number) < eligible.length || !Number.isSafeInteger(otherWeight) || (otherWeight as number) < 0) return null;
  const total = eligible.reduce((sum, entry) => sum + (selection['mode'] === 'weighted_random' ? entry.weight : 1), otherWeight as number);
  if (!Number.isSafeInteger(total) || total <= 0 || (selection['mode'] === 'weighted_random' && selection['winner'] !== null && !eligible.find((entry) => entry.key === selection['winner'])?.weight)) return null;
  return { competitionId: result['competitionId'], rankingId: result['rankingId'], generation: result['generation'] as number, winner: selection['winner'] as string | null, mode: selection['mode'] as RevealResult['mode'], eligible, eligibleCount: eligibleCount as number, otherWeight: otherWeight as number };
}
export class RevealController {
  private current: RevealResult | null = null;
  constructor(private readonly rankingId: string, private readonly onResult: (result: RevealResult | null) => void, private readonly competitionId?: string) {}
  handleEnvelope(envelope: unknown): boolean {
    const result = parseRevealResult(envelope);
    if (!result || result.rankingId !== this.rankingId || (this.competitionId && result.competitionId !== this.competitionId) || (this.current && result.competitionId !== this.current.competitionId) || (this.current && result.competitionId === this.current.competitionId && result.generation < this.current.generation)) return false;
    if (this.current && result.competitionId === this.current.competitionId && result.generation === this.current.generation) return false;
    this.current = result; this.onResult(result); return true;
  }
  clear(): void { this.current = null; this.onResult(null); }
  dispose(): void { this.current = null; }
}
