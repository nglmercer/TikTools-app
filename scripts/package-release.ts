import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { validateCliBuildInfo } from './lib/cli-release.ts';
import { cp, mkdir, rm, stat } from 'node:fs/promises';
import { basename, join, resolve } from 'node:path';

const repositoryRoot = resolve(import.meta.dir, '..');
const supportedPlatforms = {
  'windows-x86_64': {
    archiveExtension: '.zip',
    binaryName: 'tiktools-desktop.exe',
    rustTarget: 'x86_64-pc-windows-msvc',
  },
  'linux-x86_64': {
    archiveExtension: '.tar.gz',
    binaryName: 'tiktools-desktop',
    rustTarget: 'x86_64-unknown-linux-gnu',
  },
} as const;

type ReleasePlatform = keyof typeof supportedPlatforms;

function fail(message: string): never {
  throw new Error(`Release packaging failed: ${message}`);
}

function requiredEnvironment(name: string): string {
  const value = process.env[name]?.trim();
  if (!value) fail(`${name} is required`);
  return value;
}

function run(command: string, args: string[]): string {
  const result = Bun.spawnSync({
    cmd: [command, ...args],
    cwd: repositoryRoot,
    stdout: 'pipe',
    stderr: 'inherit',
  });
  const output = result.stdout ? new TextDecoder().decode(result.stdout) : '';
  if (!result.success) fail(`${command} ${args.join(' ')} exited with code ${result.exitCode}`);
  return output;
}

async function isFile(path: string): Promise<boolean> {
  const info = await stat(path).catch(() => null);
  return info?.isFile() ?? false;
}

function isSupportedPlatform(value: string): value is ReleasePlatform {
  return Object.hasOwn(supportedPlatforms, value);
}

const tag = requiredEnvironment('RELEASE_TAG');
if (!/^v[^/]+$/.test(tag)) fail(`RELEASE_TAG must be a Git tag such as v0.1.0, got ${tag}`);
const version = tag.slice(1);
const platformValue = requiredEnvironment('RELEASE_PLATFORM');
if (!isSupportedPlatform(platformValue)) {
  fail(`RELEASE_PLATFORM must be one of ${Object.keys(supportedPlatforms).join(', ')}`);
}
const platform = supportedPlatforms[platformValue];

const webRoot = resolve(repositoryRoot, 'dist', 'web');
const webIndex = join(webRoot, 'index.html');
if (!(await isFile(webIndex))) {
  fail(`frontend output is missing ${webIndex}; run bun scripts/task.ts frontend-build first`);
}

const binaryPath = resolve(
  repositoryRoot,
  process.env.RELEASE_BINARY?.trim() || join('target', 'release', platform.binaryName),
);
if (!(await isFile(binaryPath))) {
  fail(`compiled desktop binary is missing ${binaryPath}`);
}

// Packaging may also be invoked outside release CI. Check the tag, manifests,
// Packager config, and actual executable before touching existing staging.
const versionCheck = Bun.spawn([process.execPath, 'scripts/check-version.ts', '--binary'], {
  cwd: repositoryRoot,
  env: { ...process.env, RELEASE_TAG: tag, RELEASE_BINARY: binaryPath },
  stdin: 'ignore',
  stdout: 'inherit',
  stderr: 'inherit',
});
if (await versionCheck.exited !== 0) fail('release version verification failed');

const cliName = platformValue === 'windows-x86_64' ? 'tiktools.exe' : 'tiktools';
const cliPath = resolve(repositoryRoot, process.env.RELEASE_CLI_BINARY?.trim() || join('target', 'release', cliName));
if (!(await isFile(cliPath))) fail(`compiled headless CLI is missing ${cliPath}; build tiktools-cli with --features remote-control`);
const revision = run('git', ['rev-parse', 'HEAD']).trim();
if (run('git', ['status', '--porcelain', '--untracked-files=no']).trim()) fail('commit tracked source changes before packaging a release');
const cliInfo: unknown = JSON.parse(run(cliPath, ['--build-info']));
validateCliBuildInfo(cliInfo, version, revision);
const cliManifest = { ...cliInfo, stagedBinarySha256: createHash('sha256').update(await readFile(cliPath)).digest('hex') };

const releaseDirectory = resolve(repositoryRoot, 'release');
const stagingDirectory = join(releaseDirectory, 'staging');
const bundleName = 'TikTools';
const bundleDirectory = join(stagingDirectory, bundleName);
const archiveName = `TikTools-${version}-${platformValue}${platform.archiveExtension}`;
const archivePath = join(releaseDirectory, archiveName);

await rm(bundleDirectory, { recursive: true, force: true });
await rm(archivePath, { force: true });
await mkdir(bundleDirectory, { recursive: true });

await cp(binaryPath, join(bundleDirectory, platform.binaryName));
await cp(cliPath, join(bundleDirectory, cliName));
await writeFile(join(bundleDirectory, 'cli-build.json'), `${JSON.stringify(cliManifest, null, 2)}\n`);
// Plugins are not bundled: they install separately (marketplace or .plugin
// files) and their sources live in the plugins-example-tiktools repository.
await cp(webRoot, join(bundleDirectory, 'web'), { recursive: true });
for (const file of ['LICENSE', 'README.md']) {
  const source = resolve(repositoryRoot, file);
  if (!(await isFile(source))) fail(`release documentation is missing ${source}`);
  await cp(source, join(bundleDirectory, file));
}

if (platformValue === 'windows-x86_64') {
  run('tar', ['-a', '-c', '-f', archivePath, '-C', stagingDirectory, bundleName]);
} else {
  run('tar', ['-czf', archivePath, '-C', stagingDirectory, bundleName]);
}

const archiveListing = platformValue === 'windows-x86_64'
  ? run('tar', ['-tf', archivePath])
  : run('tar', ['-tzf', archivePath]);
const listingEntries = archiveListing.split(/\r?\n/).map((entry) => entry.replaceAll('\\', '/'));
const expectedEntries = [
  `${bundleName}/${platform.binaryName}`,
  `${bundleName}/web/index.html`,
  `${bundleName}/LICENSE`,
  `${bundleName}/README.md`,
];
for (const expectedEntry of expectedEntries) {
  if (!listingEntries.some((entry) => entry === expectedEntry)) {
    fail(`archive ${archivePath} does not contain ${expectedEntry}`);
  }
}

// Verify the archive itself, not only the pre-archive staging directory. This
// catches layout regressions caused by the platform tar/ZIP implementation.
const extractedDirectory = join(stagingDirectory, 'verify-extracted');
await rm(extractedDirectory, { recursive: true, force: true });
await mkdir(extractedDirectory, { recursive: true });
run('tar', platformValue === 'windows-x86_64'
  ? ['-xf', archivePath, '-C', extractedDirectory]
  : ['-xzf', archivePath, '-C', extractedDirectory]);
const extractedBundle = join(extractedDirectory, bundleName);
for (const relative of [
  platform.binaryName,
  'web/index.html',
  'LICENSE',
  'README.md',
]) {
  if (!(await isFile(join(extractedBundle, relative)))) {
    fail(`extracted archive is missing ${bundleName}/${relative}`);
  }
}

console.log(`Created ${basename(archivePath)}`);
console.log(`Package root: ${bundleDirectory}`);
console.log(`Archive: ${archivePath}`);
