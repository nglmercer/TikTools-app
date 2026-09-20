import { prepareDevelopmentPlugins, repositoryRoot } from './dev-plugins.ts';
import {
  allocateFreePort,
  devUrl,
  findDesktopOwners,
  isControlHostRunning,
  waitForHttp,
} from './lib/dev-launch.ts';

function fail(message: string): never {
  throw new Error(`Development startup failed: ${message}`);
}

function refuseStaleOwners(context: string): void {
  const owners = findDesktopOwners();
  if (owners.length > 0) {
    fail(
      `a TikTools desktop is already running (PID ${owners.join(', ')}) ${context}. ` +
        'Close the existing desktop before starting dev, so the new Vite server ' +
        'is never mixed with an old desktop — even one whose IPC server is down.',
    );
  }
}

const existing = await isControlHostRunning();
if (existing) {
  fail(
    'a TikTools desktop/control host is already running on the production IPC endpoint. ' +
      'Close the existing desktop (or stop the standalone host) before starting dev, ' +
      'so the new Vite server is never mixed with an old desktop or stale control IPC host.',
  );
}
refuseStaleOwners('on this machine');

// Never guess the Vite port: allocate a free loopback port first, then run
// Vite with strictPort so it fails loudly instead of drifting to another
// port while the desktop still points at the guessed URL.
const webPort = await allocateFreePort();
const actualDevUrl = devUrl(webPort);

let developmentPluginRoot = process.env.TIKTOOLS_DEV_PLUGINS_DIR;
if (!developmentPluginRoot && process.env.TIKTOOLS_SKIP_DEV_PLUGINS !== '1') {
  developmentPluginRoot = await prepareDevelopmentPlugins();
}

console.log(`Starting owned Vite dev server (${actualDevUrl})...`);
const vite = Bun.spawn({
  cmd: [process.execPath, 'run', 'serve:web'],
  cwd: repositoryRoot,
  env: { ...process.env, TIKTOOLS_WEB_PORT: String(webPort) },
  stdout: 'inherit',
  stderr: 'inherit',
});

const viteAlive = (): boolean => vite.exitCode === null;
const killVite = (): void => {
  try {
    vite.kill();
  } catch {
    // Already exited; nothing to stop.
  }
};

const shutdownSignals: Array<NodeJS.Signals> = ['SIGINT', 'SIGTERM'];
const forwardSignal = (signal: NodeJS.Signals): void => {
  vite.kill(signal);
};
for (const signal of shutdownSignals) {
  process.once(signal, forwardSignal);
}

try {
  await waitForHttp(actualDevUrl, viteAlive);
} catch (error) {
  killVite();
  throw error;
}

// Re-check immediately before launching the desktop: an old desktop
// starting concurrently must never be mixed with this new Vite server.
// The process probe catches desktops whose IPC server is already down
// (or never came up) and which the IPC check alone would miss.
if (await isControlHostRunning()) {
  killVite();
  fail(
    'a TikTools desktop/control host appeared while Vite was starting. ' +
      'Close the existing desktop before starting dev.',
  );
}
try {
  refuseStaleOwners('that appeared while Vite was starting');
} catch (error) {
  killVite();
  throw error;
}

console.log(`Vite is ready; launching the desktop host against ${actualDevUrl}...`);
const environment: Record<string, string | undefined> = {
  ...process.env,
  TIKTOOLS_DEV_URL: actualDevUrl,
  TIKTOOLS_WEB_PORT: String(webPort),
};
if (developmentPluginRoot) {
  environment.TIKTOOLS_DEV_PLUGINS_DIR = developmentPluginRoot;
}

const host = Bun.spawn({
  cmd: ['node', 'scripts/cargo-with-linker.mjs', 'run', '-p', 'tiktools-desktop'],
  cwd: repositoryRoot,
  env: environment,
  stdin: 'inherit',
  stdout: 'inherit',
  stderr: 'inherit',
});

try {
  process.exitCode = await host.exited;
} finally {
  // The Vite process is owned by this launcher: never leave it behind
  // when the desktop exits, or a stale Vite will serve the next run.
  killVite();
}
