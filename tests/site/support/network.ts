import type { Page } from '@playwright/test';

const LOCAL_SCHEME = /^(data:|blob:)/;

/** Record every request that leaves the site's own origin. Optionally block it. */
export function trackOutbound(page: Page, origin: string, options: { block?: boolean } = {}): string[] {
  const outbound: string[] = [];
  if (options.block) {
    void page.route('**/*', async (route) => {
      const url = route.request().url();
      if (LOCAL_SCHEME.test(url) || url.startsWith(origin)) await route.continue();
      else { outbound.push(url); await route.abort('blockedbyclient'); }
    });
  } else {
    page.on('request', (request) => { if (!LOCAL_SCHEME.test(request.url()) && !request.url().startsWith(origin)) outbound.push(request.url()); });
  }
  return outbound;
}

/** Page errors, collected for an end-of-test `expect(errors).toEqual([])`. */
export function trackPageErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  return errors;
}
