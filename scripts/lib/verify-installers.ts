import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { lstat, mkdtemp, open, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';

async function run(args: string[], cwd: string, timeoutMs = 600_000): Promise<string> {
  const child = Bun.spawn(args, { cwd, stdout: 'pipe', stderr: 'pipe' });
  const timer = setTimeout(() => child.kill(), timeoutMs);
  const [output, errors, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]).finally(() => clearTimeout(timer));
  if (code !== 0) throw new Error(`${basename(args[0] as string)} failed: ${errors.slice(-2000)}`);
  return output;
}

async function files(root: string, relative = ''): Promise<string[]> {
  const entries = await readdir(join(root, relative), { withFileTypes: true });
  const results: string[] = [];
  for (const entry of entries) {
    const path = join(relative, entry.name);
    if (entry.isDirectory()) results.push(...await files(root, path));
    else if (entry.isFile()) results.push(path);
    else throw new Error(`unsupported staged file type: ${path}`);
  }
  return results;
}

async function digest(path: string): Promise<string> {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path)) hash.update(chunk as Buffer);
  return hash.digest('hex');
}

async function isElf(path: string): Promise<boolean> {
  const file = await open(path, 'r');
  try {
    const header = Buffer.alloc(4);
    const { bytesRead } = await file.read(header, 0, 4, 0);
    return bytesRead === 4 && header.equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46]));
  } finally { await file.close(); }
}

async function matchesPatchedElf(source: string, packaged: string): Promise<boolean> {
  if (!await isElf(source) || !await isElf(packaged)) return false;
  const [before, after] = await Promise.all([run(['readelf', '-n', source], tmpdir()), run(['readelf', '-n', packaged], tmpdir())]);
  const buildId = (text: string) => text.match(/Build ID:\s*([0-9a-f]+)/i)?.[1]?.toLowerCase();
  const id = buildId(before);
  return !!id && id === buildId(after);
}

async function checkContents(staging: string, resources: string, binary: string, version: string, appimage: boolean): Promise<void> {
  for (const relative of ['web', 'plugins']) {
    const entries = await files(join(staging, relative));
    if (entries.length === 0) throw new Error(`staged ${relative} is empty`);
    for (const entry of entries) {
      const source = join(staging, relative, entry);
      const packaged = join(resources, relative, entry);
      if ((await lstat(packaged)).isSymbolicLink() ||
        (await digest(source) !== await digest(packaged) && !(appimage && await matchesPatchedElf(source, packaged)))) {
        throw new Error(`installer content mismatch: ${relative}/${entry}`);
      }
    }
  }
  for (const file of ['LICENSE', 'README.md']) {
    if (await digest(join(staging, file)) !== await digest(join(resources, file))) throw new Error(`installer content mismatch: ${file}`);
  }
  const stagedBinary = join(staging, basename(binary));
  if (await digest(stagedBinary) !== await digest(binary) && !(appimage && await matchesPatchedElf(stagedBinary, binary))) throw new Error('installer binary differs from staged binary');
  if ((await run([binary, '--version'], resources, 15_000)).trim() !== version) throw new Error('installer binary version does not match the release');
}

/** Inspect extracted installer contents against the already verified staging tree. */
export async function verifyInstaller(staging: string, installer: string, format: 'deb' | 'appimage' | 'nsis', version: string): Promise<void> {
  const temp = await mkdtemp(join(tmpdir(), 'tiktools-installer-check-'));
  try {
    if (format === 'deb') {
      await run(['ar', 'x', installer], temp);
      await run(['tar', '-xzf', 'data.tar.gz', '--no-same-owner', '-C', temp], temp);
      await checkContents(staging, join(temp, 'usr', 'lib', 'tiktools-desktop'), join(temp, 'usr', 'bin', 'tiktools-desktop'), version, false);
    } else if (format === 'appimage') {
      await run([installer, '--appimage-extract'], temp);
      const root = join(temp, 'squashfs-root', 'usr');
      // linuxdeploy patches the ELF RPATH, so compare its reported version.
      await checkContents(staging, join(root, 'lib', 'tiktools-desktop'), join(root, 'bin', 'tiktools-desktop'), version, true);
    } else {
      const sevenZip = Bun.which('7z') ?? join(process.env.ProgramFiles || 'C:\\Program Files', '7-Zip', '7z.exe');
      await run([sevenZip, 'x', '-y', `-o${temp}`, installer], temp);
      const matches = (await files(temp)).filter((name) => basename(name).toLowerCase() === 'tiktools-desktop.exe');
      if (matches.length !== 1) throw new Error('NSIS archive must contain exactly one desktop executable');
      const binary = join(temp, matches[0] as string);
      await checkContents(staging, join(binary, '..'), binary, version, false);
    }
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
}
