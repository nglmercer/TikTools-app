import { describe, expect, it } from 'vitest';
import { defaultPointsConfig } from '@/entities/points/model/default-config.ts';
import { isConfigDirty, normalizePointsConfig } from '../../../../../../src/web/features/configure-points/model/points-rules.ts';

describe('points rules', () => {
  it('clamps rates, bonus and level before saving', () => {
    const normalized = normalizePointsConfig({
      ...defaultPointsConfig,
      currencyName: '  ',
      pointsPerChat: -2,
      subBonusMultiplier: 900,
      pointsPerLevel: 4.6,
    });
    expect(normalized).toMatchObject({ currencyName: 'Points', pointsPerChat: 0, subBonusMultiplier: 500, pointsPerLevel: 10 });
  });

  it('detects edits against the saved config', () => {
    expect(isConfigDirty({ ...defaultPointsConfig }, defaultPointsConfig)).toBe(false);
    expect(isConfigDirty({ ...defaultPointsConfig, pointsPerJoinEnabled: true }, defaultPointsConfig)).toBe(true);
  });
});
