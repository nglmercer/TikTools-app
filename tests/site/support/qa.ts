import { writeFile } from 'node:fs/promises';
import type { Locator, Page } from '@playwright/test';

/** Review screenshots and evidence files are written only when TIKTOOLS_E2E_DEBUG=1, as in playwright.site.config.ts. */
export const qaArtifacts = process.env.TIKTOOLS_E2E_DEBUG === '1';

export function qaScreenshot(target: Page, name: string, options?: { fullPage?: boolean }): Promise<void>;
export function qaScreenshot(target: Locator, name: string): Promise<void>;
export async function qaScreenshot(target: Page | Locator, name: string, options: { fullPage?: boolean } = {}): Promise<void> {
  if (!qaArtifacts) return;
  const path = `/tmp/${name}`;
  // Only Page screenshots accept fullPage, so narrow on a Page-only member.
  if ('goto' in target) await target.screenshot({ path, ...options });
  else await target.screenshot({ path });
}

export async function qaWrite(name: string, content: string): Promise<void> {
  if (!qaArtifacts) return;
  await writeFile(`/tmp/${name}`, content);
}
