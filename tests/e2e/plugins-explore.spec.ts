import { testMessage } from '../../src/web/shared/i18n/test-message.ts';
import { expect, test } from '@playwright/test';

import { installFakeHost, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';
import { ROUTE, RPC, defined } from '../shared/index.ts';
import { SEL } from './support/selectors.ts';

const t = (key: string): string => testMessage('en', key);

/**
 * Plugins, Explore lists the Cloud catalog when the host announces `catalog.items`;
 * the rule templates dialog holds the Cloud library as a tab, and an old `?view=cloud` link opens it there.
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
  state.announcedMethods = [RPC.catalogItems, RPC.catalogMine, 'catalog.item', 'cloudLibrary.list'];
  state.catalog = {
    items: [
      wire('plugin', 'tiktools.tts', 'Voice reader'),
      wire('template', 'tpl-gift', t('automations.builtinTemplates.giftSoundTitle')),
      wire('profile', 'prof-1', 'Giftwall starter'),
    ],
    library: [
      {
        id: '00000000-0000-4000-8000-000000000001',
        kind: 'template',
        slug: 'gift',
        revision: '3',
        title: t('automations.builtinTemplates.giftSoundTitle'),
        sha256: sha,
        status: 'published',
      },
    ],
  };
  return state;
}

test('Explore shows the published plugin from the Cloud catalog', async ({ page }) => {
  const capture = await installFakeHost(page, catalogState());
  await page.goto(ROUTE.plugins);
  await waitForFrontend(page);
  await page.getByRole('tab', { name: /Explore/ }).click();
  await expect(page.locator(SEL.exploreRow)).toHaveCount(1);
  await expect(page.locator(SEL.exploreRow)).toContainText('Voice reader');
  await expect(page.getByText('marketplace', { exact: false })).toHaveCount(0);
  capture.assertClean();
});

test('Explore shows a neutral retry state when the Cloud is unreachable, and "nothing published" for an empty registry', async ({ page }) => {
  const state = catalogState();
  defined(state.catalog).error = { code: 'unavailable', message: 'cloud down', data: { cloud: { code: 'offline' } } };
  await installFakeHost(page, state);
  await page.goto(ROUTE.plugins);
  await waitForFrontend(page);
  await page.getByRole('tab', { name: /Explore/ }).click();
  await expect(page.getByText(t('exploreCatalog.unavailable'))).toBeVisible();
  await expect(page.getByRole('button', { name: t('exploreCatalog.retry') })).toBeVisible();
  delete defined(state.catalog).error;
  defined(state.catalog).items = [];
  defined(state.catalog).dataState = 'noActiveGeneration';
  await page.evaluate(
    (next) => Object.assign(window.__TIKTOOLS_E2E_STATE__, { catalog: next }),
    state.catalog,
  );
  await page.getByRole('button', { name: t('exploreCatalog.retry') }).click();
  await expect(page.getByText(t('exploreCatalog.nothingPublished'))).toBeVisible();
});

test('the templates dialog has Templates, Cloud and Import tabs, and ?view=cloud opens the Cloud one', async ({ page }) => {
  const capture = await installFakeHost(page, catalogState());
  await page.goto('/#/automations?view=cloud');
  await waitForFrontend(page);
  const dialog = page.getByRole('dialog', { name: t('automations.templates.title') });
  await expect(dialog.getByRole('tab', { name: t('importDialog.cloudTab'), selected: true })).toBeVisible();
  await expect(dialog.getByText(t('automations.builtinTemplates.giftSoundTitle'))).toBeVisible();
  await expect(page.locator('.automation-library').getByRole('tab', { name: t('importDialog.cloudTab') })).toHaveCount(0);
  await dialog.getByRole('tab', { name: 'Import' }).click();
  await expect(dialog.getByText(t('importDialog.dropTitle'))).toBeVisible();
  capture.assertClean();
});
