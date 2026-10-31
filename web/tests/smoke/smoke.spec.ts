import { expect, test } from '@playwright/test';

// SOB-8: the first screen loads, then hydrates, without a page or console error.
test('first screen loads and hydrates without errors', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });

  const response = await page.goto('/');
  expect(response?.ok()).toBe(true);

  await expect(page.locator('main')).toBeVisible();
  await expect(page.getByRole('heading', { level: 1 }).first()).toBeVisible();

  // The pages are server-rendered, so the checks above pass without JavaScript.
  // The root layout patches history.pushState in an $effect, which runs only
  // once the app has mounted. `networkidle` never settles on a cold Vite dev server.
  await page.waitForFunction(() => Object.hasOwn(history, 'pushState'));
  // Short settle for errors thrown by work started after mount.
  await page.waitForTimeout(1000);
  expect(errors).toEqual([]);
});
