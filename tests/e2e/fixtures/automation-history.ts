import type { Page } from '@playwright/test';

/**
 * Histories and reviews share one toolbar menu. On narrow screens an open record's sheet also carries direct
 * buttons (same test ids), so prefer a visible direct button and fall back to the menu entry.
 */
const menuOrder = ['adoption', 'definition', 'library', 'approval', 'review-authority', 'review-approval'] as const;
export type HistoryEntry = (typeof menuOrder)[number];

export async function openHistoryEntry(page: Page, testId: string, entry: HistoryEntry): Promise<void> {
  const direct = page.getByTestId(testId).filter({ visible: true });
  if (await direct.count()) {
    await direct.first().click();
    return;
  }
  await page.getByTestId('history-menu').click();
  await page.getByRole('menuitem').nth(menuOrder.indexOf(entry)).click();
}
