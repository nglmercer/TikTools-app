export interface CompetitionOption { id: string; label: string; commands: string[]; giftIds: string[]; imageUrl: string | null; onWin: { actionId: string; inputs: Record<string, unknown> } | null }
export interface CompetitionDefinition {
  schemaVersion: 1; id: string; name: string; kind: 'poll' | 'weighted_poll' | 'team_battle' | 'roulette' | 'random_draw' | 'weighted_draw' | 'leaderboard_challenge'; rankingId: string;
  options: CompetitionOption[]; selectionMode: 'highest_score' | 'uniform_random' | 'weighted_random';
  tiePolicy: 'manual' | 'stable' | 'random' | 'no_winner'; durationMs: number | null;
  voteLimit: number; allowSwitch: boolean; minimumParticipants: number;
  /** Leaderboard challenges only: the leader must reach this score. */
  targetScore?: number | null;
  /** Action for user winners (participant draws and leaderboard challenges). */
  onUserWin?: { actionId: string; inputs: Record<string, unknown> } | null;
}
export interface CompetitionState { lifecycle: 'draft' | 'active' | 'paused' | 'finalizing' | 'finished' | 'cancelled' | 'failed'; generation: number; deadlineMs: number | null; remainingMs: number | null }
export interface CompetitionResult { generation: number; finalizedAt: number; selection: { mode: string; winner: string | null; eligible: { key: string; weight: number }[] } }
export interface CompetitionAction { generation: number; actionId: string; status: string; reason?: string }
