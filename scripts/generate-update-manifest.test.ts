import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { describe, expect, test } from 'bun:test';
import { readWorkspaceVersion } from './bump-version.ts';

const root = resolve(import.meta.dir, '..');

describe('signed release metadata', () => {
  test.skipIf(!Bun.which('cargo-packager'))('signs actual files and rejects tampering or missing credentials', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'tiktools-update-test-'));
    const version = readWorkspaceVersion(await readFile(join(root, 'Cargo.toml'), 'utf8'));
    const env = { ...process.env, RELEASE_TAG: `v${version}`, CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD: crypto.randomUUID() };
    async function run(args: string[], variables: Record<string, string | undefined>) {
      const child = Bun.spawn(args, { cwd: root, env: variables, stdout: 'pipe', stderr: 'pipe' });
      const [, , code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
      return code;
    }
    try {
      expect(await run(['cargo', 'packager', 'signer', 'generate', '--ci', '--path', join(dir, 'fixture-key')], env)).toBe(0);
      const signedEnv = {
        ...env,
        CARGO_PACKAGER_SIGN_PRIVATE_KEY: await readFile(join(dir, 'fixture-key'), 'utf8'),
        TIKTOOLS_UPDATE_PUBLIC_KEY: await readFile(join(dir, 'fixture-key.pub'), 'utf8'),
      };
      const windows = join(dir, `TikTools-${version}-windows-x86_64-installer.exe`);
      const linux = join(dir, `TikTools-${version}-linux-x86_64-installer.AppImage`);
      await writeFile(windows, 'windows fixture');
      await writeFile(linux, 'linux fixture');
      const script = ['bun', 'scripts/generate-update-manifest.ts'];
      expect(await run([...script, '--sign', dir], { ...signedEnv, CARGO_PACKAGER_SIGN_PRIVATE_KEY: undefined })).not.toBe(0);
      expect(await run([...script, '--sign', dir], signedEnv)).toBe(0);
      expect(await run([...script, '--verify', dir], signedEnv)).toBe(0);
      const heads = join(dir, 'heads');
      const snapshot = join(dir, 'snapshot');
      await mkdir(heads);
      const stage = ['bun', 'scripts/stage-update-channels.ts', dir, heads];
      expect(await run([...stage, snapshot], signedEnv)).toBe(0);
      expect(await readFile(join(snapshot, 'stable.json'), 'utf8')).toBe(await readFile(join(dir, 'update.json'), 'utf8'));
      expect(await readFile(join(snapshot, 'beta.json'), 'utf8')).toBe(await readFile(join(dir, 'update.json'), 'utf8'));
      expect(await run(['bun', 'scripts/stage-update-channels.ts', dir, snapshot, join(dir, 'unchanged')], signedEnv)).toBe(0);
      expect(await readFile(join(dir, 'unchanged', 'stable.json'), 'utf8')).toBe(await readFile(join(snapshot, 'stable.json'), 'utf8'));
      await copyFile(join(snapshot, 'stable.json'), join(heads, 'stable.json'));
      await copyFile(join(snapshot, 'stable.json.sig'), join(heads, 'stable.json.sig'));
      await writeFile(join(heads, 'stable.json'), '{"version":"99.0.0"}');
      expect(await run([...stage, join(dir, 'rejected')], signedEnv)).not.toBe(0);
      await rm(join(heads, 'stable.json.sig'));
      expect(await run([...stage, join(dir, 'incomplete')], signedEnv)).not.toBe(0);
      await writeFile(linux, 'corrupt');
      expect(await run([...script, '--verify', dir], signedEnv)).not.toBe(0);
      await writeFile(linux, 'linux fixture');
      await writeFile(join(dir, 'update.json'), '{"version":"99.0.0"}');
      expect(await run([...script, '--verify', dir], signedEnv)).not.toBe(0);
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  }, 120_000);
});
