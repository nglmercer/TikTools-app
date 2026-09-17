import { ref } from 'vue';

import type { ControlClient } from '../platform/control-client.ts';
import { errorMessage } from '../platform/control-client.ts';
import type { PointsConfig, ViewerRecord } from '../types.ts';

export const defaultPointsConfig: PointsConfig = {
  currencyName: 'Points',
  pointsPerCoin: 1.0,
  pointsPerCoinEnabled: true,
  pointsPerShare: 3.0,
  pointsPerShareEnabled: true,
  pointsPerChat: 1.0,
  pointsPerChatEnabled: true,
  pointsPerLike: 0.1,
  pointsPerLikeEnabled: true,
  pointsPerFollow: 5.0,
  pointsPerFollowEnabled: true,
  pointsPerJoin: 0.5,
  pointsPerJoinEnabled: false,
  subBonusMultiplier: 0.0,
  pointsPerLevel: 100,
};

export interface LeaderboardResult {
  viewers: ViewerRecord[];
}

export interface PointsChangedData {
  uniqueId: string;
  delta: number;
  totalPoints: number;
  level: number;
}

/** Points config and leaderboard. Awards arrive only via the authoritative
 * `points.changed` domain event; the legacy push is no longer subscribed. */
export function usePoints(control: ControlClient) {
  const pointsConfig = ref<PointsConfig>(defaultPointsConfig);
  const leaderboard = ref<ViewerRecord[]>([]);

  const applyAward = (uniqueId: string, totalPoints: number, level: number): boolean => {
    const index = leaderboard.value.findIndex((viewer) => viewer.uniqueId === uniqueId);
    if (index < 0) return false;
    const updated = [...leaderboard.value];
    const current = updated[index];
    if (current) {
      updated[index] = {
        ...current,
        points: totalPoints,
        level,
        lastSeen: Date.now(),
      };
    }
    leaderboard.value = updated.sort((left, right) => right.points - left.points);
    return true;
  };

  control.onPush('points-config', (message) => {
    if (message.type !== 'points-config') return;
    pointsConfig.value = message.config;
  });
  control.onPush('leaderboard', (message) => {
    if (message.type !== 'leaderboard') return;
    leaderboard.value = message.viewers;
  });

  const refresh = async (): Promise<void> => {
    try {
      const [config, board] = await Promise.all([
        control.call<PointsConfig>('points.config.get', {}),
        control.call<LeaderboardResult>('points.leaderboard', { limit: 100 }),
      ]);
      pointsConfig.value = config;
      leaderboard.value = board.viewers;
    } catch (failure) {
      console.warn(`points refresh failed: ${errorMessage(failure)}`);
    }
  };

  // Awards for viewers missing from the loaded page re-read the board so
  // new chatters appear. One in-flight refresh at a time: it re-reads the
  // full leaderboard, so concurrent triggers would only duplicate it.
  let missingViewerRefreshInFlight = false;
  const refreshMissingViewer = (): void => {
    if (missingViewerRefreshInFlight) return;
    missingViewerRefreshInFlight = true;
    void refresh().finally(() => {
      missingViewerRefreshInFlight = false;
    });
  };

  control.onTopic<PointsChangedData>('points.changed', (data) => {
    if (!applyAward(data.uniqueId, data.totalPoints, data.level)) {
      refreshMissingViewer();
    }
  });

  const handleUpdatePointsConfig = (config: Partial<PointsConfig>): void => {
    void control
      .call<PointsConfig>('points.config.set', { ...config })
      .then((next) => {
        pointsConfig.value = next;
      })
      .catch((failure: unknown) => {
        console.warn(`points.config.set failed: ${errorMessage(failure)}`);
      });
  };

  const handleResetPoints = (uniqueId?: string): void => {
    void (async () => {
      try {
        await control.call('points.reset', uniqueId === undefined ? {} : { uniqueId });
        const board = await control.call<LeaderboardResult>('points.leaderboard', {
          limit: 100,
        });
        leaderboard.value = board.viewers;
      } catch (failure) {
        console.warn(`points.reset failed: ${errorMessage(failure)}`);
      }
    })();
  };

  const handleAdjustPoints = (uniqueId: string, delta: number): void => {
    // The leaderboard patch arrives via the points.changed event.
    void control.call('points.adjust', { uniqueId, delta }).catch((failure: unknown) => {
      console.warn(`points.adjust failed: ${errorMessage(failure)}`);
    });
  };

  const leaderboardPointsFor = (handle: string): number | undefined => {
    const clean = handle.trim().replace(/^@/, '').toLowerCase();
    const viewer = leaderboard.value.find(
      (entry) => entry.uniqueId.trim().replace(/^@/, '').toLowerCase() === clean,
    );
    return viewer?.points;
  };

  return {
    pointsConfig,
    leaderboard,
    leaderboardPointsFor,
    handleUpdatePointsConfig,
    handleResetPoints,
    handleAdjustPoints,
    refresh,
  };
}
