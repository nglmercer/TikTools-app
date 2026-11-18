import { testMessage } from '../../src/web/shared/i18n/test-message.ts';

const t = (key: string): string => testMessage('en', key);
import { expect, test } from '@playwright/test';
import type { Page } from '@playwright/test';
import { assertNoUnhandledCalls, installFakeHost, readHostState, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';

async function openOverlays(page: Page) {
  const capture = await installFakeHost(page, emptyState(), { theme: 'dark' });
  await page.goto('/');
  await waitForFrontend(page);
  await page.getByRole('link', { name: 'Overlays', exact: true }).click();
  await expect(page.getByRole('listbox', { name: 'Layers', exact: true })).toBeVisible();
  return capture;
}

const layers = (page: Page) => page.getByRole('listbox', { name: 'Layers', exact: true });

test('layer text edits update the preview, save, and survive navigation', async ({ page }) => {
  const capture = await openOverlays(page);
  await layers(page).getByRole('option', { name: /Message Text/ }).click();
  const content = page.getByRole('textbox', { name: 'Content', exact: true });
  await expect(content).toHaveValue('just followed!');
  await content.fill('Thanks {{name}}!');
  await expect(page.locator('.widget-layer-text.is-message')).toHaveText('Thanks Viewer Name!');
  await page.getByRole('button', { name: t('overlays.editor.save'), exact: true }).click();
  await expect(page.getByText(t('overlays.editor.saved'), { exact: true })).toBeVisible();
  expect((await readHostState(page)).calls.some((call) => call.method === 'app.state.set' && call.params['key'] === 'widgets.design.follow')).toBe(true);
  await page.getByRole('link', { name: t('navigation.feed'), exact: true }).click();
  await page.getByRole('link', { name: 'Overlays', exact: true }).click();
  await layers(page).getByRole('option', { name: /Message Text/ }).click();
  await expect(content).toHaveValue('Thanks {{name}}!');
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('every widget exposes its default layers in the inspector and preview', async ({ page }) => {
  const capture = await openOverlays(page);
  for (const [kind, fields] of [
    [t('overlays.kinds.follow.name'), [t('overlays.editor.fields.name'), 'Username', 'Message']],
    ['Gift alert', [t('overlays.editor.fields.name'), 'Message', t('overlays.editor.fields.count'), 'Diamonds']],
    [t('overlays.kinds.chat.name'), [t('overlays.editor.fields.name'), 'Message']],
    [t('overlays.kinds.share.name'), [t('overlays.editor.fields.name'), 'Username', 'Message']],
    [t('overlays.kinds.subscribe.name'), [t('overlays.editor.fields.name'), 'Username', 'Message']],
  ] as const) {
    await page.getByRole('option', { name: kind, exact: true }).click();
    for (const field of fields) {
      await layers(page).getByRole('option', { name: new RegExp(field + ' Text') }).click();
      await expect(page.getByRole('textbox', { name: 'Content', exact: true })).toBeVisible();
    }
    await expect(page.locator('.widget-host').getByText('Viewer Name', { exact: true }).first()).toBeVisible();
  }
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('gift layers can be removed, undone, and restored with redo', async ({ page }) => {
  const capture = await openOverlays(page);
  await page.getByRole('option', { name: 'Gift alert', exact: true }).click();
  await layers(page).getByRole('option', { name: /Diamonds Text/ }).click();
  await page.getByRole('button', { name: t('overlays.editor.removeLayer'), exact: true }).click();
  await expect(layers(page).getByRole('option', { name: /Diamonds Text/ })).toHaveCount(0);
  await expect(page.locator('.widget-layer-text.is-diamonds, .gift-diamonds')).toHaveCount(0);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(layers(page).getByRole('option', { name: /Diamonds Text/ })).toBeVisible();
  await expect(page.locator('.widget-layer-text.is-diamonds, .gift-diamonds')).toContainText('5,000');
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(page.locator('.widget-layer-text.is-diamonds, .gift-diamonds')).toHaveCount(0);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('card styles preview, discard, save per widget, and reset', async ({ page }) => {
  const capture = await openOverlays(page);
  await page.getByRole('tab', { name: 'Card', exact: true }).click();
  const background = page.getByRole('textbox', { name: 'Background', exact: true });
  await background.fill('#112233');
  await background.press('Enter');
  await expect(page.locator('.follow-card')).toHaveCSS('background-color', 'rgb(17, 34, 51)');
  await page.getByRole('button', { name: t('overlays.editor.discard'), exact: true }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Discard', exact: true }).click();
  await expect(page.locator('.follow-card')).toHaveCSS('background-color', 'rgb(22, 22, 29)');
  await background.fill('#112233');
  await background.press('Enter');
  await page.getByRole('button', { name: t('overlays.editor.save'), exact: true }).click();
  await expect(page.getByText(t('overlays.editor.saved'), { exact: true })).toBeVisible();
  await page.getByRole('option', { name: 'Gift alert', exact: true }).click();
  await expect(page.locator('.gift-card')).toHaveCSS('background-color', 'rgb(22, 22, 29)');
  await page.getByRole('option', { name: t('overlays.kinds.follow.name'), exact: true }).click();
  await expect(background).toHaveValue('#112233');
  await page.getByRole('button', { name: t('overlays.editor.options'), exact: true }).click();
  await page.getByRole('menuitem', { name: t('overlays.editor.resetDesign'), exact: true }).click();
  await page.getByRole('button', { name: t('overlays.editor.save'), exact: true }).click();
  await expect(page.locator('.follow-card')).toHaveCSS('background-color', 'rgb(22, 22, 29)');
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('rapid replay and tab changes never place alert cards side by side', async ({ page }) => {
  await installFakeHost(page, emptyState(), { theme: 'dark' });
  await page.goto('/');
  await waitForFrontend(page);
  await page.getByRole('link', { name: 'Overlays', exact: true }).click();
  await expect(page.locator('.follow-card')).toBeVisible();
  // Observe every animation frame, including the intermediate transition
  // states that a final visibility assertion would miss.
  await page.evaluate(() => {
    const violations: string[] = [];
    const target = window as unknown as { widgetTransitionViolations: string[]; stopWidgetMonitor: () => void };
    target.widgetTransitionViolations = violations;
    let frame = 0;
    const sample = () => {
      const host = document.querySelector('.widget-host');
      const cards = host?.querySelectorAll('[role="alert"]') ?? [];
      if (cards.length > 1) violations.push('multiple cards');
      const bounds = host?.getBoundingClientRect();
      if (bounds) {
        for (const card of Array.from(cards)) {
          const rect = card.getBoundingClientRect();
          if (Math.abs(rect.x + rect.width / 2 - bounds.x - bounds.width / 2) > 2) {
            violations.push('card moved off center');
          }
        }
      }
      frame = requestAnimationFrame(sample);
    };
    sample();
    target.stopWidgetMonitor = () => cancelAnimationFrame(frame);
  });
  for (const label of [t('overlays.kinds.follow.name'), 'Gift alert', t('overlays.kinds.share.name'), t('overlays.kinds.subscribe.name'), t('overlays.kinds.follow.name')]) {
    await page.getByRole('option', { name: label, exact: true }).click();
    await expect(page.locator('.widget-host [role="alert"]')).toBeVisible();
    for (let index = 0; index < 3; index++) {
      await page.getByRole('button', { name: t('overlays.preview.replay') }).click();
      await page.waitForTimeout(150);
    }
  }
  await page.waitForTimeout(700);
  const violations = await page.evaluate(() => {
    const target = window as unknown as { widgetTransitionViolations: string[]; stopWidgetMonitor: () => void };
    target.stopWidgetMonitor();
    return target.widgetTransitionViolations;
  });
  expect(violations).toEqual([]);
  await expect(page.locator('.follow-card')).toHaveCount(1);
});

test('all widget previews render locally without a gateway and replay', async ({ page }, testInfo) => {
  const consoleCapture = await installFakeHost(page, emptyState(), { theme: 'dark' });
  const gatewayRequests: string[] = [];
  page.on('request', (request) => {
    if (request.url().includes(':17452')) gatewayRequests.push(request.url());
  });
  await page.goto('/');
  await waitForFrontend(page);
  await page.getByRole('link', { name: 'Overlays', exact: true }).click();
  await expect(page.getByRole('button', { name: t('overlays.source.copy'), exact: true })).toBeDisabled();
  for (const [label, kind, selector] of [
    [t('overlays.kinds.follow.name'), 'follow', '.follow-card'],
    ['Gift alert', 'gift', '.gift-card'],
    [t('overlays.kinds.chat.name'), 'chat', '.chat-message'],
    [t('overlays.kinds.share.name'), 'share', '.share-card'],
    [t('overlays.kinds.subscribe.name'), 'subscribe', '.subscribe-card'],
  ]) {
    await page.getByRole('option', { name: label, exact: true }).click();
    await expect(page.locator('.widget-host')).toHaveAttribute('data-widget', kind!);
    await expect(page.locator(selector!).first()).toBeVisible();
    await page.getByRole('button', { name: t('overlays.preview.replay') }).click();
    await expect(page.locator(selector!).first()).toBeVisible();
    await expect(page.locator('.widget-host')).toHaveCount(1);
  }
  // The sample must remain visible beyond the normal subscription duration.
  await page.waitForTimeout(5500);
  await expect(page.locator('.subscribe-card')).toBeVisible();
  await expect(page.locator('.overlay-preview__canvas iframe')).toHaveCount(0);
  expect(gatewayRequests).toEqual([]);
  await page.screenshot({ path: testInfo.outputPath('widgets-preview.png') });
  await page.setViewportSize({ width: 520, height: 800 });
  const preview = await page.locator('.overlay-preview__canvas').boundingBox();
  const card = await page.locator('.subscribe-card').boundingBox();
  expect(card!.width).toBeLessThanOrEqual(preview!.width);
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});
