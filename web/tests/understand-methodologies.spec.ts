import { expect, test, type Page } from '@playwright/test';

import { demoAnswers } from './helpers/demo-answers';
import { ONBOARDED_PREFS } from './helpers/fixtures';
import { installTauriStub, ipcCalls, setStubPrefs, stubPrefs } from './helpers/tauri-stub';

/**
 * `/methodologies` on the desktop path (SOB-224), through the IPC stub: the demo has no save
 * conflict to show. The page reads the settings once, when it opens, and every save writes the
 * whole object it read.
 */

const OPENED_WITH = { ...ONBOARDED_PREFS, persona: null, lang: 'fr' };
const CHANGED_ELSEWHERE = { ...ONBOARDED_PREFS, persona: 'student', lang: 'en' };

test.beforeEach(async ({ page }) => {
  const [methodologies] = await demoAnswers(page, [['list_methodologies']]);
  await installTauriStub(page, {
    responses: { list_methodologies: methodologies },
    state: { prefs: OPENED_WITH }
  });
});

async function openMethodologies(page: Page) {
  await page.goto('/methodologies');
  const cards = page.locator('li.method-card');
  await expect(cards).toHaveCount(2);
  return cards;
}

test('the page lists the methodologies and marks the default one', async ({ page }) => {
  const cards = await openMethodologies(page);

  await expect(page.getByRole('heading', { name: 'Catalogue de méthodologies' })).toBeVisible();
  await expect(cards.nth(0)).toContainText('AFNOR SPEC 2314');
  await expect(cards.nth(0)).toContainText('Par défaut');
  await expect(cards.nth(1)).toContainText('EcoLogits 2026-01');
  await expect(cards.nth(1).locator('input[type="checkbox"]')).toBeChecked();
});

// SOB-224 audit: the save sends the settings read at page load, not the current ones.
test('quirk — choosing the default method overwrites a setting changed elsewhere meanwhile', async ({
  page
}) => {
  const cards = await openMethodologies(page);

  await setStubPrefs(page, CHANGED_ELSEWHERE);
  await cards.nth(1).getByRole('button', { name: 'Définir comme par défaut' }).click();
  await expect(page.getByRole('status')).toHaveText('EcoLogits 2026-01 définie par défaut.');

  const saves = await ipcCalls(page, 'set_app_preferences');
  expect(saves).toHaveLength(1);
  expect(saves[0]?.args).toEqual({
    prefs: { ...OPENED_WITH, default_method: 'ecologits', also_show_methods: [] }
  });
  const stored = await stubPrefs(page);
  expect(stored).toMatchObject({ default_method: 'ecologits', persona: null, lang: 'fr' });
});

test('quirk — toggling "Voir aussi" overwrites a setting changed elsewhere meanwhile', async ({
  page
}) => {
  const cards = await openMethodologies(page);

  await setStubPrefs(page, CHANGED_ELSEWHERE);
  await cards.nth(1).locator('input[type="checkbox"]').uncheck();
  await expect(page.getByRole('status')).toHaveText('EcoLogits 2026-01 retirée de "Voir aussi".');

  const stored = await stubPrefs(page);
  expect(stored).toEqual({ ...OPENED_WITH, also_show_methods: [] });
});

test('the default method cannot be unchecked as a reference or chosen again', async ({ page }) => {
  const afnor = (await openMethodologies(page)).nth(0);
  const choose = afnor.getByRole('button', { name: 'Méthodo par défaut active' });
  const reference = afnor.locator('input[type="checkbox"]');

  await expect(choose).toBeDisabled();
  await expect(reference).toBeDisabled();
  await choose.click({ force: true });
  await reference.click({ force: true });
  expect(await ipcCalls(page, 'set_app_preferences')).toHaveLength(0);
});
