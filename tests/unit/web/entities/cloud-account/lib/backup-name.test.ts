import { expect, it } from 'vitest';
import { suggestBackupName } from '../../../../../../src/web/entities/cloud-account/lib/backup-name.ts';

it('builds a sortable local-time name and never exceeds the 120 character limit', () => {
  expect(suggestBackupName('Backup', new Date(2026, 10, 21, 2, 5))).toBe('Backup 2026-11-21 02:05');
  expect(suggestBackupName('x'.repeat(200), new Date(2026, 0, 1, 0, 0))).toHaveLength(120);
});
