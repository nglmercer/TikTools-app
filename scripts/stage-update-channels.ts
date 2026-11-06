/** Stage a complete, authenticated channel snapshot for a separate atomic deployment. */
import { copyFile, mkdir, mkdtemp, readFile, rename, rm, stat } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { selectUpdateChannels } from './lib/update-channel.ts';

const [candidateArg, headsArg, outputArg, ...rest] = process.argv.slice(2);
if (!candidateArg || !headsArg || !outputArg || rest.length) {
  throw new Error('usage: bun scripts/stage-update-channels.ts <candidate-directory> <current-heads-directory> <new-snapshot-directory>');
}
if (!process.env.TIKTOOLS_UPDATE_PUBLIC_KEY?.trim()) throw new Error('TIKTOOLS_UPDATE_PUBLIC_KEY is required');
const root = resolve(import.meta.dir, '..');
const candidate = resolve(candidateArg);
const heads = resolve(headsArg);
const output = resolve(outputArg);
try { await stat(output); throw new Error('snapshot destination already exists'); } catch (error) {
  if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
}
await mkdir(dirname(output), { recursive: true });
const staging = await mkdtemp(join(dirname(output), '.update-snapshot-'));
const cleanEnv = { ...process.env, CARGO_PACKAGER_SIGN_PRIVATE_KEY: undefined, CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD: undefined };
async function verify(file: string): Promise<string> {
  const child = Bun.spawn(['cargo', 'run', '--quiet', '--locked', '-p', 'tiktools-app-update', '--bin', 'verify-update', '--', 'manifest', file, `${file}.sig`], { cwd: root, env: cleanEnv, stdout: 'pipe', stderr: 'pipe' });
  const [, , code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
  if (code !== 0) throw new Error('update manifest authentication failed');
  const parsed: unknown = JSON.parse(await readFile(file, 'utf8'));
  if (!parsed || typeof parsed !== 'object' || !('version' in parsed) || typeof parsed.version !== 'string') throw new Error('missing manifest version');
  return parsed.version;
}
async function exists(file: string): Promise<boolean> {
  try { return (await stat(file)).isFile(); } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') return false;
    throw error;
  }
}
async function copyPair(source: string, destination: string) {
  await copyFile(source, destination);
  await copyFile(`${source}.sig`, `${destination}.sig`);
}
try {
  const stagedCandidate = join(staging, 'candidate.json');
  await copyPair(join(candidate, 'update.json'), stagedCandidate);
  const version = await verify(stagedCandidate);
  const current: Record<'stable' | 'beta', string | null> = { stable: null, beta: null };
  for (const channel of ['stable', 'beta'] as const) {
    const source = join(heads, `${channel}.json`);
    const [manifestExists, signatureExists] = await Promise.all([exists(source), exists(`${source}.sig`)]);
    if (manifestExists !== signatureExists) throw new Error(`incomplete ${channel} feed head`);
    if (manifestExists) {
      const destination = join(staging, `${channel}.json`);
      await copyPair(source, destination);
      current[channel] = await verify(destination);
      if (channel === 'stable' && current[channel]!.split('+', 1)[0]!.includes('-')) throw new Error('stable feed head is a prerelease');
    }
  }
  const selected = selectUpdateChannels(version, current.stable, current.beta);
  for (const channel of selected) await copyPair(stagedCandidate, join(staging, `${channel}.json`));
  await rm(stagedCandidate);
  await rm(`${stagedCandidate}.sig`);
  await rename(staging, output);
  console.log(`Staged ${selected.length ? selected.join(', ') : 'unchanged'} channels for ${version}`);
} catch (error) {
  await rm(staging, { recursive: true, force: true });
  throw error;
}
