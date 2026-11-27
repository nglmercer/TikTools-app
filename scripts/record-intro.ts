/**
 * Records the demo showcase (`?showcase=1&record=1`) from the built site and encodes it for the
 * landing page, the README and release notes. Build first: `bun scripts/task.ts site build`.
 *
 *   bun scripts/task.ts site record-intro            # English, dark theme
 *   TIKTOOLS_INTRO_LOCALE=es TIKTOOLS_INTRO_THEME=light bun scripts/task.ts site record-intro
 *
 * Output (dist/intro-video): intro.webm (raw), intro.mp4 (site, releases), intro.webp (README), poster.png.
 */
import { chromium } from '@playwright/test';
import { mkdir, readdir, rename, rm } from 'node:fs/promises';
import { resolve } from 'node:path';
import { siteConfig } from './lib/site-config.ts';
import { demoLocales } from '../src/web/demo/scenarios.ts';

const root = resolve(import.meta.dirname, '..');
const out = resolve(root, 'dist/intro-video');
const locale = process.env.TIKTOOLS_INTRO_LOCALE ?? 'en';
const theme = process.env.TIKTOOLS_INTRO_THEME ?? 'dark';
const port = Number(process.env.TIKTOOLS_INTRO_PORT ?? 4181);
if (!(demoLocales as readonly string[]).includes(locale)) throw new Error(`Unknown locale "${locale}".`);
if (theme !== 'dark' && theme !== 'light') throw new Error('TIKTOOLS_INTRO_THEME must be dark or light.');
if (!Number.isInteger(port) || port < 1024 || port > 65535) throw new Error('Invalid TIKTOOLS_INTRO_PORT.');
const { base } = siteConfig();
const origin = `http://127.0.0.1:${port}`;

async function run(command: string[]): Promise<void> {
  const child = Bun.spawn(command, { stdout: 'inherit', stderr: 'inherit' });
  if (await child.exited !== 0) throw new Error(`Command failed: ${command.join(' ')}`);
}

async function waitFor(url: string, ms: number): Promise<void> {
  const deadline = Date.now() + ms;
  while (Date.now() < deadline) {
    if ((await fetch(url).catch(() => null))?.ok) return;
    await Bun.sleep(200);
  }
  throw new Error(`Timed out waiting for ${url}. Run \`bun scripts/task.ts site build\` first.`);
}

await rm(out, { recursive: true, force: true });
await mkdir(out, { recursive: true });
const preview = Bun.spawn([process.execPath, resolve(root, 'scripts/preview-site.ts')], { env: { ...process.env, TIKTOOLS_SITE_PORT: String(port) }, stdout: 'ignore', stderr: 'inherit' });
try {
  await waitFor(`${origin}${base}demo/`, 15_000);
  const browser = await chromium.launch();
  const started = Date.now();
  const context = await browser.newContext({ viewport: { width: 1280, height: 720 }, recordVideo: { dir: out, size: { width: 1280, height: 720 } } });
  const page = await context.newPage();
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto(`${origin}${base}demo/?showcase=1&record=1&locale=${locale}&theme=${theme}&instance=intro-video`);
  // Trim the blank load: the first frame worth keeping is the first spotlight.
  await page.locator('.showcase[data-on="true"]').waitFor({ timeout: 30_000 });
  const lead = Date.now() - started;
  await page.waitForFunction(() => document.documentElement.dataset.showcaseDone === 'true', null, { timeout: 90_000 });
  const total = Date.now() - started;
  await page.screenshot({ path: resolve(out, 'poster.png') });
  await context.close();
  await browser.close();
  if (errors.length) throw new Error(`The page reported errors: ${errors.join('; ')}`);
  const recorded = (await readdir(out)).find((name) => name.endsWith('.webm'));
  if (!recorded) throw new Error('Playwright did not write a video.');
  await rename(resolve(out, recorded), resolve(out, 'intro.webm'));
  const input = resolve(out, 'intro.webm');
  const trim = ['-ss', (lead / 1000).toFixed(2), '-t', ((total - lead) / 1000 + 0.4).toFixed(2)];
  await run(['ffmpeg', '-y', '-loglevel', 'error', ...trim, '-i', input, '-c:v', 'libx264', '-crf', '20', '-pix_fmt', 'yuv420p', '-movflags', '+faststart', '-an', resolve(out, 'intro.mp4')]);
  await run(['ffmpeg', '-y', '-loglevel', 'error', ...trim, '-i', input, '-vf', 'fps=12,scale=800:-1:flags=lanczos', '-c:v', 'libwebp', '-loop', '0', '-q:v', '50', '-compression_level', '6', '-an', resolve(out, 'intro.webp')]);
  for (const name of ['intro.mp4', 'intro.webp', 'poster.png']) console.log(`${name}: ${(Bun.file(resolve(out, name)).size / 1024).toFixed(0)} KiB`);
  console.log(`Recorded ${((total - lead) / 1000).toFixed(1)} s into ${out}`);
} finally {
  preview.kill();
}
