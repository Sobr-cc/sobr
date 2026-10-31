import { expect, type Locator, type Page } from '@playwright/test';

/** Runs an estimate from the home page and returns the ledger signature that closes the result. */
export async function estimateFromHome(page: Page, prompt: string): Promise<Locator> {
  await page.locator('form.composer textarea').fill(prompt);
  await page.getByRole('button', { name: /Estimer l'impact/ }).click();
  const signature = page.locator('div.signature');
  await expect(signature).toBeVisible({ timeout: 15_000 });
  return signature;
}
