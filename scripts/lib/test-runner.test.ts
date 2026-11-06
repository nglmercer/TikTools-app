import { describe, expect, test } from 'bun:test';

import {
  BUN_AREA_DIRS,
  FAST_BUN_ROOTS,
  buildTestCommands,
  bunTargetDir,
  parseCargoMetadata,
  parseFeatureList,
  parseTestArgs,
  pluginTestRoots,
  runTargetedTests,
  rustFilterMatchesList,
  validateRustCrate,
} from './test-runner.ts';

const CRATES = ['tiktools-cli', 'tiktools-core', 'tiktools-desktop'];

describe('parseTestArgs defaults', () => {
  test('no args selects fast units', () => {
    expect(parseTestArgs([])).toEqual({ area: 'default', watch: false, changed: false });
  });

  test('--changed selects changed fast units', () => {
    expect(parseTestArgs(['--changed'])).toEqual({ area: 'default', watch: false, changed: true });
  });
});

describe('parseTestArgs dispatch', () => {
  test('rust crate only', () => {
    expect(parseTestArgs(['rust', 'tiktools-core'])).toEqual({
      area: 'rust',
      crate: 'tiktools-core',
      filter: undefined,
      features: undefined,
      watch: false,
      changed: false,
    });
  });

  test('rust crate with filter', () => {
    const parsed = parseTestArgs(['rust', 'tiktools-core', 'batched_likes']);
    expect(parsed).toMatchObject({ area: 'rust', crate: 'tiktools-core', filter: 'batched_likes' });
  });

  test('rust crate with features', () => {
    const parsed = parseTestArgs(['rust', 'tiktools-core', '--features', 'native-tiktok']);
    expect(parsed).toMatchObject({ area: 'rust', crate: 'tiktools-core', features: 'native-tiktok' });
  });

  test('rust crate with filter and features in either form', () => {
    expect(parseTestArgs(['rust', 'tiktools-core', 'batched_likes', '--features', 'native-tiktok'])).toMatchObject({
      filter: 'batched_likes',
      features: 'native-tiktok',
    });
    expect(parseTestArgs(['rust', 'tiktools-core', '--features=native-tiktok'])).toMatchObject({
      features: 'native-tiktok',
    });
  });

  test('web filter and watch', () => {
    expect(parseTestArgs(['web', 'live-batch'])).toMatchObject({ area: 'web', filter: 'live-batch' });
    expect(parseTestArgs(['web', '--watch'])).toMatchObject({ area: 'web', watch: true });
    expect(parseTestArgs(['web'])).toMatchObject({ area: 'web', filter: undefined });
  });

  test('bun area', () => {
    expect(parseTestArgs(['bun', 'automation'])).toMatchObject({ area: 'bun', target: 'automation' });
  });

  test('plugin name', () => {
    expect(parseTestArgs(['plugin', 'sonicboom'])).toMatchObject({ area: 'plugin', plugin: 'sonicboom' });
  });

  test('e2e with optional filter', () => {
    expect(parseTestArgs(['e2e'])).toMatchObject({ area: 'e2e' });
    expect(parseTestArgs(['e2e', 'feed.spec.ts'])).toMatchObject({ area: 'e2e', filter: 'feed.spec.ts' });
  });
});

describe('parseTestArgs invalid targets', () => {
  test('unknown area never falls back to all tests', () => {
    expect(() => parseTestArgs(['bogus'])).toThrow(/unknown test area/);
  });

  test('rust without a crate fails', () => {
    expect(() => parseTestArgs(['rust'])).toThrow(/needs a crate/);
  });

  test('bun without a target fails', () => {
    expect(() => parseTestArgs(['bun'])).toThrow(/needs an area/);
  });

  test('plugin without a name fails', () => {
    expect(() => parseTestArgs(['plugin'])).toThrow(/needs a name/);
  });

  test('unknown flags fail instead of being ignored', () => {
    expect(() => parseTestArgs(['--bogus'])).toThrow(/unsupported flag/);
    expect(() => parseTestArgs(['web', 'x', '--bogus'])).toThrow(/unsupported flag/);
  });

  test('area-restricted flags are rejected elsewhere', () => {
    expect(() => parseTestArgs(['rust', 'tiktools-core', '--watch'])).toThrow(/--watch/);
    expect(() => parseTestArgs(['web', 'x', '--features', 'f'])).toThrow(/--features/);
    expect(() => parseTestArgs(['rust', 'tiktools-core', '--changed'])).toThrow(/--changed/);
    expect(() => parseTestArgs(['e2e', '--changed'])).toThrow(/--changed/);
  });

  test('path traversal filters are rejected', () => {
    expect(() => parseTestArgs(['web', '../secret'])).toThrow(/invalid filter/);
    expect(() => parseTestArgs(['rust', 'tiktools-core', '..'])).toThrow(/invalid filter/);
  });

  test('unsafe plugin names are rejected', () => {
    expect(() => parseTestArgs(['plugin', '../evil'])).toThrow(/invalid plugin name/);
  });

  test('empty features are rejected and kept explicit', () => {
    expect(() => parseFeatureList('')).toThrow(/needs a comma-separated value/);
    expect(() => parseFeatureList('a,')).toThrow(/invalid feature/);
    expect(parseFeatureList('native-tiktok')).toBe('native-tiktok');
  });
});

describe('workspace metadata and crate validation', () => {
  test('parseCargoMetadata extracts sorted package names', () => {
    const names = parseCargoMetadata(
      JSON.stringify({ packages: [{ name: 'b-crate' }, { name: 'a-crate' }, { name: 'b-crate' }] }),
    );
    expect(names).toEqual(['a-crate', 'b-crate']);
  });

  test('parseCargoMetadata rejects garbage and empty workspaces', () => {
    expect(() => parseCargoMetadata('not json')).toThrow(/could not parse/);
    expect(() => parseCargoMetadata(JSON.stringify({ packages: [] }))).toThrow(/no workspace crates/);
  });

  test('unknown crate fails with the workspace list', () => {
    expect(() => validateRustCrate('nope', CRATES)).toThrow(/unknown Rust crate.*tiktools-core/);
    expect(() => validateRustCrate('tiktools-core', CRATES)).not.toThrow();
  });

  test('unknown bun target fails without falling back', () => {
    expect(bunTargetDir('automation')).toBe(BUN_AREA_DIRS['automation']);
    expect(() => bunTargetDir('nope')).toThrow(/unknown bun test target/);
  });

  test('plugin roots stay under the plugin directory', () => {
    expect(pluginTestRoots('sonicboom')).toEqual([
      'plugins/sonicboom/shared',
      'plugins/sonicboom/ui/src',
      'plugins/sonicboom/tests',
    ]);
  });

  test('plugin commands run inside the external plugin checkout', () => {
    const [cmd] = buildTestCommands(
      { area: 'plugin', plugin: 'sonicboom', watch: false, changed: false },
      { pluginRoots: ['plugins/sonicboom/shared'], pluginsRepository: '/work/plugins-example-tiktools' },
    )!;
    expect(cmd?.slice(1)).toEqual(['--cwd', '/work/plugins-example-tiktools', 'test', 'plugins/sonicboom/shared']);
  });
});

describe('command dispatch', () => {
  test('default builds the fast unit commands', () => {
    const cmds = buildTestCommands({ area: 'default', watch: false, changed: false }, {});
    expect(cmds).toHaveLength(2);
    expect(cmds[0]?.slice(1, 3)).toEqual(['test', 'src/automation']);
    expect(cmds[0]).toEqual(expect.arrayContaining(FAST_BUN_ROOTS));
    expect(cmds[1]?.slice(-2)).toEqual(['vitest', 'run']);
  });

  test('rust builds a locked linker-wrapped cargo command', () => {
    const [cmd] = buildTestCommands(
      { area: 'rust', crate: 'tiktools-core', filter: 'batched_likes', features: 'native-tiktok', watch: false, changed: false },
      { workspaceCrates: CRATES },
    )!;
    expect(cmd?.slice(1)).toEqual([
      'scripts/cargo-with-linker.mjs',
      'test',
      '-p',
      'tiktools-core',
      '--locked',
      '--features',
      'native-tiktok',
      'batched_likes',
    ]);
  });

  test('rust rejects unknown crates instead of testing everything', () => {
    expect(() =>
      buildTestCommands({ area: 'rust', crate: 'nope', watch: false, changed: false }, { workspaceCrates: CRATES }),
    ).toThrow(/unknown Rust crate/);
  });

  test('web builds a vitest filter command', () => {
    const [cmd] = buildTestCommands({ area: 'web', filter: 'live-batch', watch: false, changed: false }, {})!;
    expect(cmd?.slice(-3)).toEqual(['vitest', 'run', 'live-batch']);
  });

  test('plugin with no roots fails loudly', () => {
    expect(() =>
      buildTestCommands({ area: 'plugin', plugin: 'ghost', watch: false, changed: false }, { pluginRoots: [] }),
    ).toThrow(/no bun test roots/);
  });
});

describe('rust filter existence', () => {
  const listing = [
    'test e2e_batched_likes_trigger_saved_150_goals_without_breaking_old_templates: test',
    'test other_helper: test',
  ].join('\n');

  test('substring filter matches the listing', () => {
    expect(rustFilterMatchesList(listing, 'batched_likes')).toBe(true);
  });

  test('missing filter does not match', () => {
    expect(rustFilterMatchesList(listing, 'no_such_test_xyz')).toBe(false);
  });
});

describe('failure propagation', () => {
  test('a failing runner propagates instead of reporting success', async () => {
    const seen: string[][] = [];
    await expect(
      runTargetedTests(['web', 'live-batch'], {
        root: '/repo',
        run: async (cmd) => {
          seen.push(cmd);
          throw new Error('vitest failed (exit 1)');
        },
      }),
    ).rejects.toThrow(/vitest failed/);
    expect(seen).toHaveLength(1);
  });

  test('an unknown crate fails before spawning anything', async () => {
    const seen: string[][] = [];
    await expect(
      runTargetedTests(['rust', 'nope-crate'], {
        root: '/repo',
        workspaceCrates: CRATES,
        run: async (cmd) => {
          seen.push(cmd);
        },
      }),
    ).rejects.toThrow(/unknown Rust crate/);
    expect(seen).toHaveLength(0);
  });

  test('a missing rust filter fails via the list verification', async () => {
    await expect(
      runTargetedTests(['rust', 'tiktools-core', 'no_such_test_xyz'], {
        root: '/repo',
        workspaceCrates: CRATES,
        verifyRustFilter: async () => {
          throw new Error('no Rust tests match "no_such_test_xyz" in crate "tiktools-core"');
        },
        run: async () => {
          throw new Error('must not spawn after a missing filter');
        },
      }),
    ).rejects.toThrow(/no Rust tests match/);
  });
});
