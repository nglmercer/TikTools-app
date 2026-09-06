import { prepareDevelopmentPlugins, repositoryRoot } from './dev-plugins';

// Packaged-asset startup path: builds dist/web, serves it through the
// tiktools://app custom protocol, and launches the desktop host without
// TIKTOOLS_DEV_URL. Use this to validate release-like frontend loading;
// normal development (`bun run start`) uses the Vite dev server instead.
function run(command: string, args: string[]): void {
  const result = Bun.spawnSync({
    cmd: [command, ...args],
    cwd: repositoryRoot,
    stdout: 'inherit',
    stderr: 'inherit',
  });
  if (!result.success) {
    throw new Error(`${command} ${args.join(' ')} exited with code ${result.exitCode}`);
  }
}

run(process.execPath, ['run', 'build:web']);

let developmentPluginRoot = process.env.TIKTOOLS_DEV_PLUGINS_DIR;
if (!developmentPluginRoot && process.env.TIKTOOLS_SKIP_DEV_PLUGINS !== '1') {
  developmentPluginRoot = await prepareDevelopmentPlugins();
}

const environment = { ...process.env };
if (developmentPluginRoot) {
  environment.TIKTOOLS_DEV_PLUGINS_DIR = developmentPluginRoot;
}

const host = Bun.spawn({
  cmd: ['cargo', 'run', '-p', 'tiktools-desktop'],
  cwd: repositoryRoot,
  env: environment,
  stdin: 'inherit',
  stdout: 'inherit',
  stderr: 'inherit',
});

process.exitCode = await host.exited;
