import { expect, test } from '@playwright/test';

import { demoAnswers } from './helpers/demo-answers';
import {
  installTauriStub,
  ipcCalls,
  ONBOARDED_PREFS,
  setStubPrefs,
  stubPrefs
} from './helpers/tauri-stub';

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

test('the page lists the methodologies and marks the default one', async ({ page }) => {
  await page.goto('/methodologies');

  await expect(page.getByRole('heading', { name: 'Catalogue de méthodologies' })).toBeVisible();
  const cards = page.locator('li.method-card');
  await expect(cards).toHaveCount(2);
  await expect(cards.nth(0)).toContainText('AFNOR SPEC 2314');
  await expect(cards.nth(0)).toContainText('Par défaut');
  await expect(cards.nth(1)).toContainText('EcoLogits 2026-01');
  await expect(cards.nth(1).locator('input[type="checkbox"]')).toBeChecked();
});

// SOB-224 audit: the save sends the settings read at page load, not the current ones.
test('quirk — choosing the default method overwrites a setting changed elsewhere meanwhile', async ({
  page
}) => {
  await page.goto('/methodologies');
  await expect(page.locator('li.method-card')).toHaveCount(2);

  await setStubPrefs(page, CHANGED_ELSEWHERE);
  await page
    .locator('li.method-card')
    .nth(1)
    .getByRole('button', { name: 'Définir comme par défaut' })
    .click();
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
  await page.goto('/methodologies');
  await expect(page.locator('li.method-card')).toHaveCount(2);

  await setStubPrefs(page, CHANGED_ELSEWHERE);
  await page.locator('li.method-card').nth(1).locator('input[type="checkbox"]').uncheck();
  await expect(page.getByRole('status')).toHaveText('EcoLogits 2026-01 retirée de "Voir aussi".');

  const stored = await stubPrefs(page);
  expect(stored).toEqual({ ...OPENED_WITH, also_show_methods: [] });
});

test('the default method cannot be unchecked as a reference or chosen again', async ({ page }) => {
  await page.goto('/methodologies');
  const afnor = page.locator('li.method-card').nth(0);

  await expect(afnor.getByRole('button', { name: 'Méthodo par défaut active' })).toBeDisabled();
  await expect(afnor.locator('input[type="checkbox"]')).toBeDisabled();
  expect(await ipcCalls(page, 'set_app_preferences')).toHaveLength(0);
});
