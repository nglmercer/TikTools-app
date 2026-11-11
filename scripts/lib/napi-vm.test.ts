import { afterAll, beforeAll, describe, expect, test } from 'bun:test';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  compileNapiVmGuest,
  nativeStageInvocations,
  planNativeSources,
  prepareNativeAddons,
  readNativeAddons,
  stageNativeBuildArgs,
  stageNativeExecutable,
  typescriptCompilerArgs,
} from './napi-vm.ts';
import { resolveRustTarget } from './plugin-targets.ts';

const linux = resolveRustTarget('x86_64-unknown-linux-gnu');
const windows = resolveRustTarget('x86_64-pc-windows-msvc');
const manifestWith = (extra: Record<string, unknown> = {}) => ({
  id: 'fixture.native',
  runtime: 'napi-vm',
  nativeAddons: [{ package: 'fixture-native', root: 'node_modules/fixture-native' }],
  ...extra,
});

let workspace = '';
let repositoryRoot = '';
let packageDirectory = '';

beforeAll(async () => {
  workspace = await mkdtemp(join(tmpdir(), 'napi-vm-'));
  repositoryRoot = join(workspace, 'TikTools');
  packageDirectory = join(workspace, 'plugins-repo', 'plugins', 'fixture');
  await mkdir(repositoryRoot, { recursive: true });
  await mkdir(packageDirectory, { recursive: true });
  await writeFile(join(packageDirectory, 'tsconfig.json'), '{}');
});

afterAll(async () => {
  await rm(workspace, { recursive: true, force: true });
});

type Call = { command: string; args: string[] };
function recorder() {
  const execs: Call[] = [];
  const cargos: string[][] = [];
  return {
    execs,
    cargos,
    exec: (command: string, args: string[]) => void execs.push({ command, args }),
    cargo: (args: string[]) => void cargos.push(args),
  };
}

async function addSibling(): Promise<void> {
  await mkdir(join(workspace, 'fixture-native'), { recursive: true });
  await writeFile(join(workspace, 'fixture-native', 'package.json'), '{}');
}
async function removeSibling(): Promise<void> {
  await rm(join(workspace, 'fixture-native'), { recursive: true, force: true });
}
const lockfile = () => join(packageDirectory, 'native-libs.lock.json');

describe('guest compilation', () => {
  test('uses the repository tsc and ambient types from TikTools', async () => {
    const calls = recorder();
    await compileNapiVmGuest({ repositoryRoot, packageDirectory, manifestPath: 'plugin.json', exec: calls.exec });
    expect(calls.execs).toEqual([
      { command: 'node', args: typescriptCompilerArgs(repositoryRoot, join(packageDirectory, 'tsconfig.json')) },
    ]);
    expect(calls.execs[0]?.args.slice(1)).toEqual([
      '-p',
      join(packageDirectory, 'tsconfig.json'),
      '--typeRoots',
      join(repositoryRoot, 'node_modules', '@types'),
    ]);
  });

  test('a package without tsconfig.json fails and names its manifest', async () => {
    const calls = recorder();
    await expect(
      compileNapiVmGuest({ repositoryRoot, packageDirectory: workspace, manifestPath: '/m/plugin.json', exec: calls.exec }),
    ).rejects.toThrow('/m/plugin.json is napi-vm but has no tsconfig.json');
    expect(calls.execs).toHaveLength(0);
  });
});

describe('native addon declarations', () => {
  test('no declaration means nothing to stage', () => {
    expect(readNativeAddons({}, 'plugin.json')).toEqual([]);
  });

  test('rejects malformed, traversing and non-package entries', () => {
    for (const bad of [
      [{ package: 'x' }],
      [{ package: '../x', root: 'node_modules/x' }],
      [{ package: 'x', root: '../out' }],
      [{ package: 'x', root: '/abs' }],
      [{ package: 'x', root: 'a\\b' }],
      [null],
      'nope',
    ]) {
      expect(() => readNativeAddons({ nativeAddons: bad }, 'plugin.json')).toThrow(/plugin\.json has a malformed nativeAddons/);
    }
    expect(readNativeAddons({ nativeAddons: [{ package: '@scope/x', root: 'node_modules/@scope/x' }] }, 'p')).toEqual([
      { packageName: '@scope/x', root: 'node_modules/@scope/x' },
    ]);
  });
});

describe('source selection', () => {
  test('host build with every sibling present uses the siblings', () => {
    expect(planNativeSources({ missingSiblings: [], canFetch: true, crossTarget: false })).toEqual({ kind: 'sibling' });
  });

  test('a missing sibling falls back to the pinned lockfile, else is unavailable', () => {
    expect(planNativeSources({ missingSiblings: ['a'], canFetch: true, crossTarget: false })).toEqual({ kind: 'fetch' });
    expect(planNativeSources({ missingSiblings: ['a'], canFetch: false, crossTarget: false })).toEqual({
      kind: 'unavailable',
      reason: 'missing-sibling',
      missing: ['a'],
    });
  });

  test('a cross-target build never uses the host sibling', () => {
    expect(planNativeSources({ missingSiblings: [], canFetch: true, crossTarget: true })).toEqual({ kind: 'fetch' });
    expect(planNativeSources({ missingSiblings: [], canFetch: false, crossTarget: true })).toMatchObject({
      kind: 'unavailable',
      reason: 'cross-target',
    });
  });

  test('staging tool path follows the host, not the build target', () => {
    expect(stageNativeExecutable('/r', 'debug', linux)).toBe('/r/target/debug/tiktools-plugin-stage-native');
    expect(stageNativeExecutable('/r', 'release', windows)).toBe('/r/target/release/tiktools-plugin-stage-native.exe');
    expect(stageNativeBuildArgs('debug')).not.toContain('--release');
    expect(stageNativeBuildArgs('release')).toContain('--release');
  });

  test('fetch invocations carry the explicit napi target', () => {
    const [args] = nativeStageInvocations({
      plan: { kind: 'fetch' },
      repositoryRoot: '/r',
      packageDirectory: '/p',
      manifestPath: '/p/plugin.json',
      lockfilePath: '/p/native-libs.lock.json',
      addons: [{ packageName: 'x', root: 'node_modules/x' }],
      target: windows,
    });
    expect(args).toEqual([
      '--provider', 'all', '--manifest', '/p/plugin.json', '--plugin-dir', '/p',
      '--lockfile', '/p/native-libs.lock.json', '--target', 'win32-x64-msvc', '--overwrite',
    ]);
  });
});

describe('native addon preparation with fixtures', () => {
  const base = () => ({
    repositoryRoot,
    packageDirectory,
    manifestPath: join(packageDirectory, 'plugin.json'),
    host: linux,
    profile: 'debug' as const,
  });

  test('a manifest without native addons runs nothing', async () => {
    const calls = recorder();
    const result = await prepareNativeAddons({ ...base(), manifest: { id: 'x' }, target: linux, cargo: calls.cargo, exec: calls.exec });
    expect(result).toEqual({ status: 'none' });
    expect(calls.cargos).toHaveLength(0);
    expect(calls.execs).toHaveLength(0);
  });

  test('host target with a sibling checkout stages from the sibling', async () => {
    await addSibling();
    const calls = recorder();
    const result = await prepareNativeAddons({ ...base(), manifest: manifestWith(), target: linux, cargo: calls.cargo, exec: calls.exec });
    expect(result).toEqual({ status: 'staged', source: 'sibling' });
    expect(calls.cargos).toEqual([stageNativeBuildArgs('debug')]);
    expect(calls.execs).toEqual([
      {
        command: join(repositoryRoot, 'target', 'debug', 'tiktools-plugin-stage-native'),
        args: [
          '--package', 'fixture-native', '--source', join(workspace, 'fixture-native'),
          '--plugin-dir', packageDirectory, '--root', 'node_modules/fixture-native', '--overwrite',
        ],
      },
    ]);
  });

  test('a different target ignores the sibling and fetches that target from the lockfile', async () => {
    await addSibling();
    await writeFile(lockfile(), '{}');
    try {
      const calls = recorder();
      const manifest = manifestWith({ nativeLibs: [{ package: 'fixture-native', version: '1.0.0' }] });
      const result = await prepareNativeAddons({ ...base(), manifest, target: windows, cargo: calls.cargo, exec: calls.exec });
      expect(result).toEqual({ status: 'staged', source: 'fetch' });
      expect(calls.execs).toHaveLength(1);
      expect(calls.execs[0]?.args).toContain('--target');
      expect(calls.execs[0]?.args).toContain('win32-x64-msvc');
    } finally {
      await rm(lockfile(), { force: true });
    }
  });

  test('a different target without a lockfile is unavailable and builds nothing', async () => {
    await addSibling();
    const calls = recorder();
    const result = await prepareNativeAddons({ ...base(), manifest: manifestWith(), target: windows, cargo: calls.cargo, exec: calls.exec });
    expect(result).toMatchObject({ status: 'unavailable', reason: 'cross-target' });
    expect(calls.cargos).toHaveLength(0);
    expect(calls.execs).toHaveLength(0);
  });

  test('a missing dependency is reported with its package and builds nothing', async () => {
    await removeSibling();
    const calls = recorder();
    const result = await prepareNativeAddons({ ...base(), manifest: manifestWith(), target: linux, cargo: calls.cargo, exec: calls.exec });
    expect(result).toEqual({ status: 'unavailable', reason: 'missing-sibling', missing: ['fixture-native'] });
    expect(calls.cargos).toHaveLength(0);
    expect(calls.execs).toHaveLength(0);
  });

  test('a missing dependency with a pinned lockfile fetches the host binaries', async () => {
    await removeSibling();
    await writeFile(lockfile(), '{}');
    try {
      const calls = recorder();
      const manifest = manifestWith({ nativeLibs: [{ package: 'fixture-native', version: '1.0.0' }] });
      const result = await prepareNativeAddons({ ...base(), manifest, target: linux, cargo: calls.cargo, exec: calls.exec });
      expect(result).toEqual({ status: 'staged', source: 'fetch' });
      expect(calls.execs[0]?.args).toContain('linux-x64-gnu');
    } finally {
      await rm(lockfile(), { force: true });
    }
  });

  test('nativeLibs without a lockfile is not fetchable', async () => {
    await removeSibling();
    const calls = recorder();
    const manifest = manifestWith({ nativeLibs: [{ package: 'fixture-native', version: '1.0.0' }] });
    const result = await prepareNativeAddons({ ...base(), manifest, target: linux, cargo: calls.cargo, exec: calls.exec });
    expect(result).toMatchObject({ status: 'unavailable', reason: 'missing-sibling' });
  });
});
