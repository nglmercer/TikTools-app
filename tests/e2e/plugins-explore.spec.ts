import { expect, test } from '@playwright/test';

import { installFakeHost, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';

/**
 * Plugins, Explore lists the Cloud catalog when the host announces `catalog.items`; the rule templates
 * dialog holds the Cloud library as a tab, and an old `?view=cloud` link opens it there.
 */

const sha = 'a'.repeat(64);
const wire = (kind: string, id: string, title: string) => ({
  identity: { kind, id },
  presentation: { title, summary: `${title} summary`, publisher: 'tiktools', categories: [] },
  compatibility: { targets: [] },
  releaseRef: kind === 'plugin' ? { type: 'pluginRelease', version: '1.4.2' } : { type: 'documentRevision', revision: 3, sha256: sha },
  publication: { yanked: false },
});

function catalogState() {
  const state = emptyState();
  state.announcedMethods = ['catalog.items', 'catalog.mine', 'catalog.item', 'cloudLibrary.list'];
  state.catalog = {
    items: [wire('plugin', 'tiktools.tts', 'Voice reader'), wire('template', 'tpl-gift', 'Gift sound alert'), wire('profile', 'prof-1', 'Giftwall starter')],
    library: [{ id: '00000000-0000-4000-8000-000000000001', kind: 'template', slug: 'gift', revision: '3', title: 'Gift sound alert', sha256: sha, status: 'published' }],
  };
  return state;
}

test('Explore shows plugins, templates and profiles from the Cloud catalog and filters by kind', async ({ page }) => {
  const capture = await installFakeHost(page, catalogState());
  await page.goto('/#/plugins');
  await waitForFrontend(page);
  await page.getByRole('tab', { name: /Explore/ }).click();
  await expect(page.locator('.explore-list__row')).toHaveCount(3);
  await expect(page.getByText('marketplace', { exact: false })).toHaveCount(0);
  await page.getByRole('button', { name: 'Templates' }).click();
  await expect(page.locator('.explore-list__row')).toHaveCount(1);
  await expect(page.locator('.explore-list__row')).toContainText('Gift sound alert');
  capture.assertClean();
});

test('Explore shows a neutral retry state when the Cloud is unreachable, and "nothing published" for an empty registry', async ({ page }) => {
  const state = catalogState();
  state.catalog!.error = { code: 'unavailable', message: 'cloud down', data: { cloud: { code: 'offline' } } };
  await installFakeHost(page, state);
  await page.goto('/#/plugins');
  await waitForFrontend(page);
  await page.getByRole('tab', { name: /Explore/ }).click();
  await expect(page.getByText('The Cloud catalog is not available')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Try again' })).toBeVisible();
  delete state.catalog!.error;
  state.catalog!.items = [];
  state.catalog!.dataState = 'noActiveGeneration';
  await page.evaluate((next) => Object.assign((window as unknown as { __TIKTOOLS_E2E_STATE__: object }).__TIKTOOLS_E2E_STATE__, { catalog: next }), state.catalog);
  await page.getByRole('button', { name: 'Try again' }).click();
  await expect(page.getByText('Nothing published yet')).toBeVisible();
});

test('the templates dialog has Templates, Cloud and Import tabs, and ?view=cloud opens the Cloud one', async ({ page }) => {
  const capture = await installFakeHost(page, catalogState());
  await page.goto('/#/automations?view=cloud');
  await waitForFrontend(page);
  const dialog = page.getByRole('dialog', { name: 'Rule templates' });
  await expect(dialog.getByRole('tab', { name: 'Cloud', selected: true })).toBeVisible();
  await expect(dialog.getByText('Gift sound alert')).toBeVisible();
  await expect(page.locator('.automation-library').getByRole('tab', { name: 'Cloud' })).toHaveCount(0);
  await dialog.getByRole('tab', { name: 'Import' }).click();
  await expect(dialog.getByText('Drop a file here')).toBeVisible();
  capture.assertClean();
});
