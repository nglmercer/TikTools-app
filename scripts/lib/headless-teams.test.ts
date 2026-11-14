import { describe, expect, test } from 'bun:test';
import { headlessTeamsOptions } from './headless-teams.ts';

describe('headless artifact acceptance arguments', () => {
  test('requires an existing-artifact selection rather than implicit build', () => {
    expect(() => headlessTeamsOptions([], '/repo')).toThrow('--binary');
    expect(() => headlessTeamsOptions(['--binary', 'target/debug/tiktools'], '/repo')).toThrow('absolute');
    expect(() => headlessTeamsOptions(['--binary', '/bin/tiktools', '--unknown', 'x'], '/repo')).toThrow('usage');
    expect(() => headlessTeamsOptions(['--binary', '/a', '--binary', '/b'], '/repo')).toThrow('usage');
  });
  test('supports a separately pinned private release origin and report', () => {
    expect(headlessTeamsOptions(['--binary', '/bin/tiktools', '--backend', '/backend', '--report', '/out/report.json', '--cloud-port', '45443'], '/repo'))
      .toEqual({ binary: '/bin/tiktools', backend: '/backend', report: '/out/report.json', cloudPort: '45443' });
    expect(() => headlessTeamsOptions(['--binary', '/bin/tiktools', '--cloud-port', '0'], '/repo')).toThrow('1024');
  });
});
