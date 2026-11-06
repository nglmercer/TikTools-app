import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { normalizeVersion, readWorkspaceVersion } from './bump-version.ts';

const repositoryRoot = resolve(import.meta.dir, '..');

export function validateVersions(cargo: string, packageVersion: unknown, tag?: string, binary?: string): string {
  const version = normalizeVersion(cargo);
  if (version !== cargo) throw new Error(`Cargo version must be an unprefixed semver: ${cargo}`);
  if (packageVersion !== version) throw new Error(`package.json version ${String(packageVersion)} does not match Cargo ${version}`);
  if (tag !== undefined) {
    if (!tag.startsWith('v') || normalizeVersion(tag) !== version) {
      throw new Error(`release tag ${tag} does not match Cargo ${version}`);
    }
  }
  if (binary !== undefined && binary !== version) {
    throw new Error(`desktop binary version ${JSON.stringify(binary)} does not match Cargo ${version}`);
  }
  return version;
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  if (args.length > 1 || (args.length === 1 && args[0] !== '--binary')) throw new Error('expected no arguments or --binary');
  const cargo = readWorkspaceVersion(await readFile(resolve(repositoryRoot, 'Cargo.toml'), 'utf8'));
  const packageJson = JSON.parse(await readFile(resolve(repositoryRoot, 'package.json'), 'utf8')) as { version?: unknown };
  const packager = await readFile(resolve(repositoryRoot, 'Packager.toml'), 'utf8');
  const packagerVersion = packager.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (packagerVersion !== cargo) throw new Error(`Packager.toml version ${String(packagerVersion)} does not match Cargo ${cargo}`);
  let binary: string | undefined;
  if (args.includes('--binary')) {
    const path = resolve(repositoryRoot, process.env.RELEASE_BINARY || `target/release/tiktools-desktop${process.platform === 'win32' ? '.exe' : ''}`);
    const child = Bun.spawn([path, '--version'], { stdout: 'pipe', stderr: 'pipe' });
    const timer = setTimeout(() => child.kill(), 15_000);
    const [output, errors, exitCode] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]).finally(() => clearTimeout(timer));
    if (exitCode !== 0) throw new Error(`desktop version command failed (${exitCode}): ${errors}`);
    binary = output.trim();
  }
  console.log(`Version check passed: ${validateVersions(cargo, packageJson.version, process.env.RELEASE_TAG, binary)}`);
}

if (import.meta.main) {
  try { await main(); } catch (error) {
    console.error(`Version check failed: ${(error as Error).message}`);
    process.exit(1);
  }
}
