/** Create or verify a signed upstream-format manifest from release artifacts. */
import { lstat, readFile, readdir, writeFile } from 'node:fs/promises';
import { basename, join, resolve } from 'node:path';
import { normalizeVersion } from './bump-version.ts';

const root = resolve(import.meta.dir, '..');
const [mode, directoryArg, ...rest] = process.argv.slice(2);
if ((mode !== '--sign' && mode !== '--verify') || !directoryArg || rest.length) {
  throw new Error('usage: bun scripts/generate-update-manifest.ts <--sign|--verify> <release-assets-directory>');
}
const tag = process.env.RELEASE_TAG;
if (!tag?.startsWith('v')) throw new Error('RELEASE_TAG must be a v-prefixed SemVer tag');
const version = normalizeVersion(tag);
const publicKey = process.env.TIKTOOLS_UPDATE_PUBLIC_KEY;
if (!publicKey?.trim()) throw new Error('TIKTOOLS_UPDATE_PUBLIC_KEY is required');
if (mode === '--sign' && (!process.env.CARGO_PACKAGER_SIGN_PRIVATE_KEY || !process.env.CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD)) {
  throw new Error('protected signing key and password are required');
}

async function run(args: string[], env: Record<string, string | undefined>): Promise<void> {
  const child = Bun.spawn(args, { cwd: root, env, stdout: 'pipe', stderr: 'pipe' });
  // Never relay signer output: errors must not disclose signing credentials.
  const [, , code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
  if (code !== 0) throw new Error(`release ${args.includes('signer') ? 'signing' : 'verification'} command failed`);
}

const safeEnv = { ...process.env, CARGO_PACKAGER_SIGN_PRIVATE_KEY: undefined, CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD: undefined };
await run([process.execPath, 'scripts/check-version.ts'], safeEnv);
const directory = resolve(directoryArg);
const names = await readdir(directory);
const specs = [
  { target: 'windows-x86_64', extension: '.exe', format: 'nsis' },
  { target: 'linux-x86_64', extension: '.AppImage', format: 'appimage' },
] as const;
const selected = await Promise.all(specs.map(async (spec) => {
  const prefix = `TikTools-${version}-${spec.target}-installer`;
  const matches = names.filter((name) => name === `${prefix}${spec.extension}`);
  if (matches.length !== 1) throw new Error(`missing or ambiguous ${spec.target} update artifact`);
  const path = join(directory, matches[0] as string);
  const info = await lstat(path);
  if (!info.isFile() || info.size === 0 || info.size > 512 * 1024 * 1024) throw new Error(`invalid ${spec.target} update artifact`);
  return { ...spec, path };
}));

if (mode === '--sign') {
  for (const artifact of selected) {
    await run(['cargo', 'packager', 'signer', 'sign', artifact.path], process.env);
  }
}
const platforms: Record<string, { url: string; signature: string; format: string }> = {};
for (const artifact of selected) {
  const signature = (await readFile(`${artifact.path}.sig`, 'utf8')).trim();
  if (!signature || signature.length > 16 * 1024) throw new Error(`invalid ${artifact.target} signature`);
  await run(['cargo', 'run', '--quiet', '--locked', '-p', 'tiktools-app-update', '--bin', 'verify-update', '--', 'package', artifact.path, `${artifact.path}.sig`], safeEnv);
  platforms[artifact.target] = {
    url: `https://github.com/nglmercer/TikTools-app/releases/download/${encodeURIComponent(tag)}/${encodeURIComponent(basename(artifact.path))}`,
    signature,
    format: artifact.format,
  };
}
const manifest = `${JSON.stringify({ version, platforms }, null, 2)}\n`;
const manifestPath = join(directory, 'update.json');
if (mode === '--sign') {
  await writeFile(manifestPath, manifest, { flag: 'wx' });
  await run(['cargo', 'packager', 'signer', 'sign', manifestPath], process.env);
} else if (await readFile(manifestPath, 'utf8') !== manifest) {
  throw new Error('manifest does not match the signed artifacts and release tag');
}
await run(['cargo', 'run', '--quiet', '--locked', '-p', 'tiktools-app-update', '--bin', 'verify-update', '--', 'manifest', manifestPath, `${manifestPath}.sig`], safeEnv);
console.log(`Verified signed update metadata and ${selected.length} artifacts for ${tag}`);
