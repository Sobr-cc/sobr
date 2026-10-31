import { expect, test } from '@playwright/test';

import { demoAnswers } from './helpers/demo-answers';
import { estimateFromHome } from './helpers/estimate';
import { ONBOARDED_PREFS } from './helpers/fixtures';
import { installTauriStub, ipcCalls } from './helpers/tauri-stub';

/**
 * Home `/` on the desktop path (SOB-224), through the IPC stub: the "Voir aussi" panel needs the
 * preferences, which the layout only loads inside Tauri. The estimates are the engine results
 * of the demo fixtures, with the ledger id the desktop app would return.
 */

const ESTIMATE = { req: { model_id: 'gpt-4o-mini', tokens_in: 1200, tokens_out_estimated: 800 } };
const AUDIT_ID = 7;

test.beforeEach(async ({ page }) => {
  const [models, datacenters, methodologies, main, reference] = await demoAnswers(page, [
    ['list_models'],
    ['list_datacenters'],
    ['list_methodologies'],
    ['estimate_prompt', ESTIMATE],
    ['estimate_for_comparison', { ...ESTIMATE, method: 'ecologits' }]
  ]);
  await installTauriStub(page, {
    responses: {
      list_models: models,
      list_datacenters: datacenters,
      list_methodologies: methodologies,
      estimate_prompt: { ...main, audit_id: AUDIT_ID },
      estimate_for_comparison: reference
    },
    state: { prefs: ONBOARDED_PREFS }
  });
  await page.goto('/');
  await estimateFromHome(page, 'Explique-moi la photosynthèse en trois phrases.');
});

test('the "Voir aussi" panel runs the reference methodology and says every calculation is audited', async ({
  page
}) => {
  await expect(page.locator('section.also-show')).toContainText('Tous les calculs sont audités.');

  const calls = await ipcCalls(page, 'estimate_for_comparison');
  expect(calls).toHaveLength(1);
  expect(calls[0]?.args).toMatchObject({ method: 'ecologits' });
});

// SOB-224 audit: the panel says "audited" above and "not logged" on each card.
test('quirk — each reference estimate is marked not logged under a panel that says all are audited', async ({
  page
}) => {
  await expect(page.locator('.also-card-note')).toHaveText('estimation éphémère (non journalisée)');
  await expect(page.getByText('Tous les calculs sont audités.')).toBeVisible();
});

// SOB-224 audit: the label is fixed text; the chain is not checked when the result shows.
test('quirk — "Chaîne intègre" is shown without verifying the ledger', async ({ page }) => {
  const signature = page.locator('div.signature');

  await expect(signature.locator('a.hash')).toHaveText(`#${AUDIT_ID} · seed 42`);
  await expect(signature.locator('a.hash')).toHaveAttribute('href', `/journal?focus=${AUDIT_ID}`);
  await expect(signature).toContainText('Chaîne intègre');
  expect(await ipcCalls(page, 'verify_audit')).toHaveLength(0);
});
