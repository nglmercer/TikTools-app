import { prepareDevelopmentPlugins, repositoryRoot } from './dev-plugins';

const webPort = Number(process.env.TIKTOOLS_WEB_PORT ?? 3000);
const devUrl = `http://127.0.0.1:${webPort}`;

function fail(message: string): never {
  throw new Error(`Development startup failed: ${message}`);
}

async function waitForHttp(url: string, attempts = 100, delayMs = 100): Promise<void> {
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      const response = await fetch(url);
      // Any HTTP response (even 404) proves Vite is accepting connections.
      if (response) {
        await response.body?.cancel().catch(() => {});
        return;
      }
    } catch {
      // Vite is not up yet; keep waiting within the bounded retry budget.
    }
    await Bun.sleep(delayMs);
  }
  fail(`Vite dev server did not become ready at ${url}`);
}

let developmentPluginRoot = process.env.TIKTOOLS_DEV_PLUGINS_DIR;
if (!developmentPluginRoot && process.env.TIKTOOLS_SKIP_DEV_PLUGINS !== '1') {
  developmentPluginRoot = await prepareDevelopmentPlugins();
}

console.log(`Starting Vite dev server (${devUrl})...`);
const vite = Bun.spawn({
  cmd: [process.execPath, 'run', 'serve:web'],
  cwd: repositoryRoot,
  stdout: 'inherit',
  stderr: 'inherit',
});

const shutdownSignals: Array<NodeJS.Signals> = ['SIGINT', 'SIGTERM'];
const forwardSignal = (signal: NodeJS.Signals): void => {
  vite.kill(signal);
};
for (const signal of shutdownSignals) {
  process.once(signal, forwardSignal);
}

try {
  await waitForHttp(devUrl);
} catch (error) {
  vite.kill();
  throw error;
}

console.log(`Vite is ready; launching the desktop host against ${devUrl}...`);
const environment = {
  ...process.env,
  TIKTOOLS_DEV_URL: devUrl,
};
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
vite.kill();
