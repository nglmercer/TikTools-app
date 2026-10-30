/** Version 1 ranking control contracts. Scores are authoritative in Rust. */
export interface MetricDescriptor { source: string; metric: string; description: string; unit: string }
export interface RankingSource { source: string; metric: string; weight: number; enabled: boolean; giftMultipliers?: Record<string, number> }
export interface RankingDefinition {
  schemaVersion: 1;
  id: string;
  creatorId: string;
  roomId: string;
  groupBy: 'user' | 'gift' | 'event_type' | 'option' | 'team' | 'custom_key';
  window: { type: 'session' } | { type: 'rolling'; durationMs: number; maxEvents: number } | { type: 'daily'; utcOffsetMinutes: number };
  sources: RankingSource[];
  limit: number;
  capacity: number;
  userLimits?: { maxContributions?: number | null; cooldownMs?: number | null; maxUsers: number } | null;
}
