/**
 * Host/plugin interop: the REAL `PluginWebviewHost` from `src/web` (bundled
 * at test time, never imported by the plugin) drives the REAL compiled
 * plugin UI inside a sandboxed iframe over `postMessage`. This pins the
 * versioned broker envelope on both sides at once — a drift on either
 * side fails here, not in production.
 */
import { execFileSync } from 'node:child_process';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { expect, test, type Page } from '@playwright/test';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');

let shimPath: string;

test.beforeAll(() => {
  // Bundle the real host shim for the browser through the harness entry
  // (which assigns the global). The shim is dependency-free.
  const dir = mkdtempSync(join(tmpdir(), 'tiktools-shim-'));
  shimPath = join(dir, 'shim.js');
  execFileSync(
    'bun',
    [
      'build',
      'plugins/sonicboom/ui/tests/harness-entry.ts',
      '--format=iife',
      `--outfile=${shimPath}`,
    ],
    { cwd: repoRoot, stdio: 'pipe' },
  );
});

interface HarnessCall {
  method: string;
  params: Record<string, unknown>;
}

async function installHarness(page: Page, uiUrl: string): Promise<string[]> {
  const errors: string[] = [];
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });
  page.on('pageerror', (error) => {
    errors.push(error instanceof Error ? error.message : String(error));
  });
  await page.setContent(
    '<!doctype html><html><body><div id="mount"></div></body></html>',
  );
  await page.addScriptTag({ path: shimPath });
  await page.evaluate((frameSrc) => {
    const w = window as unknown as {
      __HARNESS_CALLS__: HarnessCall[];
      __HARNESS_SETTINGS__: Record<string, unknown>;
    };
    w.__HARNESS_CALLS__ = [];
    w.__HARNESS_SETTINGS__ = {
      serverUrl: 'http://127.0.0.1:17842',
      apiToken: '',
      tts: { enabled: true, language: 'en', defaultVoice: 'M1' },
    };
    const calls = w.__HARNESS_CALLS__;
    const backend = {
      getSettings: async () => {
        calls.push({ method: 'settings.get', params: {} });
        return { ...w.__HARNESS_SETTINGS__ };
      },
      setSettings: async (values: Record<string, unknown>) => {
        calls.push({ method: 'settings.set', params: { values } });
        w.__HARNESS_SETTINGS__ = { ...values };
        return { ...values };
      },
      executeAction: async (action: string, config: Record<string, unknown>) => {
        calls.push({ method: 'actions.execute', params: { action, config } });
        return {
          actionType: action,
          ok: true,
          summary: 'spoken',
          logs: ['POST /api/tts/play 200'],
          durationMs: 1,
          error: null,
        };
      },
      getOptions: async (source: string, refresh: boolean) => {
        calls.push({ method: 'options.get', params: { source, refresh } });
        if (source.endsWith(':voice')) {
          return {
            options: [
              { value: 'M1', label: 'M1' },
              { value: 'F2', label: 'F2' },
            ],
            selected: null,
          };
        }
        return {
          options: [
            { value: 'Speakers', label: 'Speakers' },
            { value: 'Headphones', label: 'Headphones' },
          ],
          selected: 'Speakers',
        };
      },
      getLocale: async () => 'en',
      getTheme: async () => 'dark',
    };
    const frame = document.createElement('iframe');
    frame.id = 'plugin-frame';
    frame.setAttribute('sandbox', 'allow-scripts');
    frame.src = frameSrc;
    document.getElementById('mount')?.appendChild(frame);
    const shim = (window as unknown as { TikToolsShim: typeof import('../../../../src/web/plugin-ui/plugin-webview-host.ts') }).TikToolsShim;
    const host = new shim.PluginWebviewHost(frame.contentWindow, {
      pluginId: 'sonicboom.server',
      backend,
      postToFrame: (envelope) => frame.contentWindow?.postMessage(envelope, '*'),
    });
    window.addEventListener('message', (event) => {
      if (event.source !== frame.contentWindow) return;
      void host.handleFrameMessage(event.data, event.source);
    });
  }, uiUrl);
  return errors;
}

async function harnessCalls(page: Page): Promise<HarnessCall[]> {
  return page.evaluate(() => {
    const w = window as unknown as { __HARNESS_CALLS__: HarnessCall[] };
    return w.__HARNESS_CALLS__;
  });
}

function assertClean(errors: string[]): void {
  if (errors.length > 0) {
    throw new Error(`Unexpected interop console/page errors:\n${errors.join('\n')}`);
  }
}

test('preview server answers cross-origin like every asset host must', async ({
  request,
  baseURL,
}) => {
  // The specs below load the UI in an opaque-origin iframe, which requires
  // `Access-Control-Allow-Origin` on every asset — provided here by the
  // `vite preview` headers. That config must never be mistaken for
  // production coverage: the desktop custom-protocol server and the dev
  // middleware have to send the same header, pinned by the Rust
  // asset-server tests (opaque origins are CORS-checked on custom
  // protocols too, as observed on WebKitGTK).
  const response = await request.get(`${baseURL}/`);
  expect(response.headers()['access-control-allow-origin']).toBe('*');
});

test('real host shim boots the real plugin UI over postMessage', async ({ page, baseURL }) => {
  const errors = await installHarness(page, `${baseURL}/`);
  const frame = page.frameLocator('#plugin-frame');

  await expect(frame.getByLabel('Default voice')).toBeVisible({ timeout: 10_000 });
  await expect(frame.getByText('2 voices')).toBeVisible();
  await expect(frame.getByLabel('Audio output').first()).toHaveValue('Speakers');

  const methods = (await harnessCalls(page)).map((call) => call.method);
  expect(methods).toContain('settings.get');
  expect(methods).toContain('options.get');

  assertClean(errors);
});

test('speech and settings round-trip through the real shim', async ({ page, baseURL }) => {
  const errors = await installHarness(page, `${baseURL}/`);
  const frame = page.frameLocator('#plugin-frame');

  await expect(frame.getByText('2 voices')).toBeVisible({ timeout: 10_000 });
  await frame.getByLabel('Voice', { exact: true }).selectOption('F2');
  await frame.getByLabel('Text', { exact: true }).fill('interop hello');
  await frame.getByRole('button', { name: 'Play' }).click();
  await expect(frame.getByRole('log')).toContainText('spoken', { timeout: 5_000 });

  const executed = (await harnessCalls(page)).filter((call) => call.method === 'actions.execute');
  expect(executed.length).toBeGreaterThanOrEqual(1);
  expect(executed[0]?.params['action']).toBe('sonicboom.server.speak');
  expect(executed[0]?.params['config']).toMatchObject({ text: 'interop hello', voice: 'F2' });

  const volume = frame.getByLabel(/Volume/);
  await volume.fill('0.7');
  await expect
    .poll(
      async () =>
        (await harnessCalls(page)).filter((call) => call.method === 'settings.set').length,
      { timeout: 5_000 },
    )
    .toBeGreaterThanOrEqual(1);

  assertClean(errors);
});
