import { expect, test } from '@playwright/test';
import { assertNoUnhandledCalls, installFakeHost } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';

test('text editing uses the same template defaults as the dashboard preview', async ({ page }, testInfo) => {
  await installFakeHost(page, emptyState(), { theme: 'dark' });
  await page.goto('/');
  await page.getByRole('button', { name: 'Widgets', exact: true }).click();
  const edit = page.getByRole('button', { name: /Edit design/ });
  const dialog = page.getByRole('dialog');
  await edit.click();
  const contentTab = dialog.getByRole('tab', { name: 'Content', exact: true });
  const stylesTab = dialog.getByRole('tab', { name: 'Style', exact: true });
  const layoutTab = dialog.getByRole('tab', { name: 'Layout', exact: true });
  // The schema drives both previews, so every default template line is
  // visible when there is no saved hiddenText override.
  await expect(dialog.getByLabel('Heading', { exact: true })).toBeVisible();
  await expect(dialog.getByLabel('Viewer name', { exact: true })).toBeVisible();
  await expect(dialog.getByLabel('Username', { exact: true })).toBeVisible();
  await expect(dialog.getByLabel('Message', { exact: true })).toBeVisible();
  await expect(dialog.locator('.follow-kicker')).toBeVisible();
  await expect(dialog.locator('.follow-name')).toBeVisible();
  await expect(dialog.locator('.follow-handle')).toBeVisible();
  await dialog.getByRole('button', { name: 'Badge', exact: true }).click();
  await expect(dialog.getByLabel('Heading', { exact: true })).toBeVisible();
  await expect(dialog.getByRole('button', { name: 'Add field', exact: true })).toHaveCount(0);
  await dialog.getByRole('button', { name: 'Alert', exact: true }).click();
  await stylesTab.focus();
  await page.keyboard.press('ArrowRight');
  await expect(layoutTab).toBeFocused();
  await expect(layoutTab).toHaveAttribute('aria-selected', 'true');
  await expect(dialog.locator('details')).toHaveCount(0);
  await expect(dialog.getByLabel('Background', { exact: true })).toBeHidden();
  await contentTab.click();
  await expect(dialog.getByLabel('Username', { exact: true })).toBeVisible();
  await expect(dialog.locator('.follow-card')).toHaveCSS('opacity', '1');
  await page.screenshot({ path: testInfo.outputPath('templates-tab.png') });
  await expect(dialog.getByLabel('Heading', { exact: true })).toHaveValue('New follower');
  await expect(dialog.locator('.follow-kicker')).toHaveText('New follower');
  await dialog.getByLabel('Heading', { exact: true }).fill('Welcome!');
  await dialog.getByLabel('Message', { exact: true }).fill('Thanks {{name}}!');
  await dialog.getByLabel('Username', { exact: true }).fill('');
  await expect(dialog.locator('.follow-action')).toHaveText('Thanks Viewer Name!');
  await expect(dialog.locator('.follow-handle')).toHaveCount(0);
  await stylesTab.click();
  await expect(dialog.getByLabel('Heading', { exact: true })).toBeHidden();
  await contentTab.click();
  await expect(dialog.getByLabel('Heading', { exact: true })).toHaveValue('Welcome!');
  await dialog.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.locator('.follow-kicker')).toHaveText('Welcome!');
  await edit.click();
  // Customized and untouched default fields both stay visible on reopen.
  await expect(dialog.getByLabel('Heading', { exact: true })).toHaveValue('Welcome!');
  await expect(dialog.getByLabel('Viewer name', { exact: true })).toBeVisible();
  await dialog.getByRole('button', { name: 'Badge', exact: true }).click();
  await expect(dialog.getByLabel('Heading', { exact: true })).toHaveValue('Welcome!');
  await dialog.getByRole('button', { name: 'Alert', exact: true }).click();
  await dialog.getByLabel('Heading', { exact: true }).fill('Discard this');
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.locator('.follow-kicker')).toHaveText('Welcome!');
  await edit.click();
  await dialog.getByRole('button', { name: 'Design options', exact: true }).click();
  await dialog.getByRole('menuitem', { name: 'Reset design' }).click();
  await expect(dialog.locator('.follow-handle')).toHaveCount(1);
  await expect(dialog.getByLabel('Heading', { exact: true })).toHaveValue('New follower');
  await expect(dialog.locator('.follow-kicker')).toHaveText('New follower');
  await expect(dialog.getByLabel('Message', { exact: true })).toHaveValue('just followed!');
  await assertNoUnhandledCalls(page);
});

test('every template exposes the same default fields in the builder and preview', async ({ page }) => {
  await installFakeHost(page, emptyState(), { theme: 'dark' });
  await page.goto('/');
  await page.getByRole('button', { name: 'Widgets', exact: true }).click();
  const dialog = page.getByRole('dialog');
  const cases = [
    { tab: 'Followers', fields: ['Heading', 'Viewer name', 'Username', 'Message'] },
    { tab: 'Gifts', fields: ['Heading', 'Streak heading', 'Viewer name', 'Message', 'Gift count', 'Diamonds'] },
    { tab: 'Chat', fields: ['Viewer name', 'Message'] },
    { tab: 'Shared', fields: ['Heading', 'Viewer name', 'Username', 'Message'] },
    { tab: 'Subscriptions', fields: ['Heading', 'Viewer name', 'Username', 'Message'] },
  ];
  for (const { tab, fields } of cases) {
    await page.getByRole('tab', { name: tab, exact: true }).click();
    await page.getByRole('button', { name: /Edit design/ }).click();
    for (const label of fields) {
      await expect(dialog.getByLabel(label, { exact: true })).toBeVisible();
    }
    await expect(dialog.getByRole('button', { name: 'Add field', exact: true })).toHaveCount(0);
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  }
  await assertNoUnhandledCalls(page);
});

test('optional gift fields can be removed and restored from the shared template', async ({ page }) => {
  await installFakeHost(page, emptyState(), { theme: 'dark' });
  await page.goto('/');
  await page.getByRole('button', { name: 'Widgets', exact: true }).click();
  await page.getByRole('tab', { name: 'Gifts', exact: true }).click();
  const edit = page.getByRole('button', { name: /Edit design/ });
  const dialog = page.getByRole('dialog');
  await edit.click();

  await expect(dialog.getByLabel('Diamonds', { exact: true })).toBeVisible();
  await dialog.getByRole('button', { name: 'Diamonds: Remove field', exact: true }).click();
  await expect(dialog.getByLabel('Diamonds', { exact: true })).toHaveCount(0);
  await expect(dialog.locator('.gift-diamonds')).toHaveCount(0);
  await expect(dialog.getByRole('button', { name: 'Add field', exact: true })).toBeVisible();

  await dialog.getByRole('button', { name: 'Add field', exact: true }).click();
  await dialog.getByRole('menuitem', { name: 'Diamonds', exact: true }).click();
  await expect(dialog.getByLabel('Diamonds', { exact: true })).toBeVisible();
  await expect(dialog.locator('.gift-diamonds')).toHaveText('5,000 diamonds');
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await assertNoUnhandledCalls(page);
});

test('style editor previews, cancels, saves per widget and resets', async ({ page }, testInfo) => {
  const capture = await installFakeHost(page, emptyState(), { theme: 'dark' });
  await page.goto('/');
  await page.getByRole('button', { name: 'Widgets', exact: true }).click();
  const edit = page.getByRole('button', { name: /Edit design/ });
  const dialog = page.getByRole('dialog');
  await edit.click();
  await dialog.getByRole('tab', { name: 'Style', exact: true }).click();
  await dialog.getByLabel('Accent color', { exact: true }).fill('#ff00ff');
  await expect(dialog.locator('.follow-badge')).toHaveCSS('background-color', 'rgb(255, 0, 255)');
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.locator('.follow-badge')).toHaveCSS('background-color', 'rgb(34, 197, 94)');
  await edit.click();
  await dialog.getByRole('tab', { name: 'Style', exact: true }).click();
  await dialog.getByLabel('Background', { exact: true }).fill('#112233');
  await dialog.getByLabel('Corner radius').fill('32');
  await expect(dialog.locator('.follow-card')).toHaveCSS('border-radius', '32px');
  await expect(dialog.locator('.follow-card')).toHaveCSS('opacity', '1');
  await page.screenshot({ path: testInfo.outputPath('widget-editor.png') });
  await dialog.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.locator('.follow-card')).toHaveCSS('background-color', 'rgb(17, 34, 51)');
  await page.getByRole('tab', { name: 'Gifts', exact: true }).click();
  await expect(page.locator('.gift-card')).toHaveCSS('background-color', 'rgb(22, 22, 29)');
  await page.getByRole('tab', { name: 'Followers', exact: true }).click();
  await edit.click();
  await dialog.getByRole('tab', { name: 'Style', exact: true }).click();
  await expect(dialog.getByLabel('Background', { exact: true })).toHaveValue('#112233');
  await dialog.getByRole('button', { name: 'Design options', exact: true }).click();
  await dialog.getByRole('menuitem', { name: 'Reset design' }).click();
  await dialog.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.locator('.follow-card')).toHaveCSS('border-radius', '16px');
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('rapid replay and tab changes never place alert cards side by side', async ({ page }) => {
  await installFakeHost(page, emptyState(), { theme: 'dark' });
  await page.goto('/');
  await page.getByRole('button', { name: 'Widgets', exact: true }).click();
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
  for (const label of ['Followers', 'Gifts', 'Shared', 'Subscriptions', 'Followers']) {
    await page.getByRole('tab', { name: label, exact: true }).click();
    await expect(page.locator('.widget-host [role="alert"]')).toBeVisible();
    for (let index = 0; index < 3; index++) {
      await page.getByRole('button', { name: 'Replay preview' }).click();
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
  await page.getByRole('button', { name: 'Widgets', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Copy URL', exact: true })).toBeDisabled();
  for (const [label, kind, selector] of [
    ['Followers', 'follow', '.follow-card'],
    ['Gifts', 'gift', '.gift-card'],
    ['Chat', 'chat', '.chat-message'],
    ['Shared', 'share', '.share-card'],
    ['Subscriptions', 'subscribe', '.subscribe-card'],
  ]) {
    await page.getByRole('tab', { name: label, exact: true }).click();
    await expect(page.locator('.widget-host')).toHaveAttribute('data-widget', kind!);
    await expect(page.locator(selector!).first()).toBeVisible();
    await page.getByRole('button', { name: 'Replay preview' }).click();
    await expect(page.locator(selector!).first()).toBeVisible();
    await expect(page.locator('.widget-host')).toHaveCount(1);
  }
  // The sample must remain visible beyond the normal subscription duration.
  await page.waitForTimeout(5500);
  await expect(page.locator('.subscribe-card')).toBeVisible();
  await expect(page.locator('.widgets-preview-frame iframe')).toHaveCount(0);
  expect(gatewayRequests).toEqual([]);
  await page.screenshot({ path: testInfo.outputPath('widgets-preview.png') });
  await page.setViewportSize({ width: 520, height: 800 });
  const preview = await page.locator('.widgets-preview-frame').boundingBox();
  const card = await page.locator('.subscribe-card').boundingBox();
  expect(card!.width).toBeLessThanOrEqual(preview!.width);
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});
