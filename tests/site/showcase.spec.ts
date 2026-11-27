import { expect, test } from '@playwright/test';
import {
  demoUrl, expectFirstChatMessage, guideTargetSelector, instanceId, selector, timeout, trackPageErrors,
} from './support/index.ts';
import { showcaseCopy } from '../../src/web/demo/showcase/copy.ts';
import { showcaseSteps } from '../../src/web/demo/showcase/script.ts';
import { defined } from '../shared/index.ts';

/** Whole recording test: the poll may take `timeout.showcaseRun`, the rest is setup. */
const RECORDING_TEST_MS = 90_000;
/**
 * Reduced motion must not advance the showcase;
 * wait this long, then check it has not moved.
 */
const STILL_OBSERVATION_MS = 2_500;
/**
 * Speed-up for the tests that only need the tour to progress;
 * recordings use real time.
 */
const TEST_PACE = 4;

// The default config reduces motion to speed the suite up; the tour only autoplays with motion.
test.use({ reducedMotion: 'no-preference' });

test('the showcase walks the real views on its own, with a spotlight and a tooltip', async ({ page }) => {
  const errors = trackPageErrors(page);
  await page.goto(demoUrl({ showcase: true, pace: TEST_PACE, instance: instanceId('showcase', 'auto') }));
  await expect(page.locator('html')).toHaveAttribute('data-demo-showcase', 'true');
  await expect(page.locator(`${selector.showcaseLayer}[data-on="true"] ${selector.showcaseTip}`)).toHaveText(showcaseCopy.en.feed);
  await expectFirstChatMessage(page);
  // The inspector profile is opened by the registered guide action, not by synthetic clicks.
  await expect(page.locator(selector.showcaseTip)).toHaveText(showcaseCopy.en.inspector, { timeout: timeout.showcaseStep });
  await expect(page.locator(guideTargetSelector(defined(showcaseSteps.find((step) => step.id === 'inspector')).target))).toBeVisible();
  await expect(page.locator(selector.demoControls)).toBeHidden();
  expect(errors).toEqual([]);
});

test('a recording run plays every step once and flags the end', async ({ page }) => {
  test.setTimeout(RECORDING_TEST_MS);
  const errors = trackPageErrors(page);
  await page.goto(demoUrl({ showcase: true, record: true, framed: true, pace: TEST_PACE, instance: instanceId('showcase', 'record') }));
  const visited = new Set<string>();
  await expect.poll(async () => {
    const text = await page.locator(selector.showcaseTip).textContent().catch(() => null);
    if (text) visited.add(text);
    return page.locator('html').getAttribute('data-showcase-done');
  }, { timeout: timeout.showcaseRun, intervals: [timeout.showcasePoll] }).toBe('true');
  expect([...visited]).toEqual(expect.arrayContaining(showcaseSteps.map((step) => showcaseCopy.en[step.id])));
  expect(errors).toEqual([]);
});

test('reduced motion shows the first step still and waits for the viewer', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  // The fake clock lets the observation window pass without waiting for it.
  await page.clock.install();
  await page.goto(demoUrl({ showcase: true, instance: instanceId('showcase', 'still') }));
  await expect(page.locator(`${selector.showcaseLayer}[data-still="true"][data-on="true"]`)).toBeVisible();
  await expectFirstChatMessage(page);
  await page.clock.runFor(STILL_OBSERVATION_MS);
  await expect(page.locator(selector.showcaseTip)).toHaveText(showcaseCopy.en.feed);
  await expect(page.locator('html')).not.toHaveAttribute('data-showcase-done', 'true');
});

test('showcase options cannot be mixed with a tour or a focused panel', async ({ page }) => {
  await page.goto(demoUrl({ showcase: true, embed: 'feed' }));
  await expect(page.getByRole('alert')).toHaveText('The showcase runs on the feed scenario without a tour, panel or preview.');
});
