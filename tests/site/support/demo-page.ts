import { expect, type Frame, type FrameLocator, type Page } from '@playwright/test';
import { demoLimits } from '../../../src/web/demo/contract.ts';
import { demoTestId } from '../../../src/web/demo/test-ids.ts';
import { FIRST_CHAT_MESSAGE } from './constants.ts';
import { selector } from './selectors.ts';

/** Pending RPC count reported by the demo metrics output. -1 until the metrics exist. */
export async function pendingRequests(page: Page | Frame): Promise<number> {
  const text = await page.getByTestId(demoTestId.metrics).textContent();
  return text ? (JSON.parse(text) as { client?: { pending?: number } }).client?.pending ?? -1 : -1;
}

/** Every simulated RPC has completed. */
export async function expectSettled(page: Page | Frame): Promise<void> {
  await expect.poll(() => pendingRequests(page), { message: 'Every demo RPC must settle' }).toBe(0);
}

/** Simulated events played so far, from the demo metrics. */
export async function playedTicks(page: Page | Frame): Promise<number> {
  const text = await page.getByTestId(demoTestId.metrics).textContent();
  return text ? (JSON.parse(text) as { host?: { ticks?: number } }).host?.ticks ?? 0 : 0;
}

/** Text of the demo event counter after `played` simulated events, e.g. `2 / 300`. */
export const eventCountText = (played: number): string => `${played} / ${demoLimits.maxTicks}`;

export const stageFrame = (page: Page): FrameLocator => page.frameLocator(selector.stageFrame);

/** The landing's embedded demo shows its first scripted chat message. */
export async function expectFirstChatMessage(frame: FrameLocator | Page): Promise<void> {
  await expect(frame.locator(selector.liveFeed)).toContainText(FIRST_CHAT_MESSAGE);
}

/** Content frame of an on-demand documentation panel, once its app is mounted and chrome is hidden. */
export async function readyPanelFrame(page: Page): Promise<Frame> {
  const element = await page.locator(selector.panelFrame).elementHandle();
  const frame = await element?.contentFrame();
  expect(frame, 'The on-demand iframe should own a browsing context').toBeTruthy();
  await expect(frame!.locator(selector.appLayout)).toBeVisible();
  await expect(frame!.locator(selector.demoControls)).toBeHidden();
  return frame!;
}
