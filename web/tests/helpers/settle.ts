import type { Page } from '@playwright/test';

/**
 * Lets the page finish what it starts after its first render (requests, then two frames), so a
 * test can assert that something is absent without passing before the page could show it.
 */
export async function settled(page: Page): Promise<void> {
  await page.waitForLoadState('networkidle');
  await page.evaluate(
    () =>
      new Promise<void>((done) => {
        requestAnimationFrame(() => requestAnimationFrame(() => done()));
      })
  );
}
