import { expect, test } from '@playwright/test';

import { estimateFromHome } from './helpers/estimate';

/**
 * Characterization of the Understand pages in demo mode (SOB-224): what the pages do today,
 * including the behaviors the audit found. Tests named `quirk` pin a defect as it is.
 * The numbers come from the demo fixtures (`src/lib/demo/fixtures`).
 */

// The first view of a route can wait for the dev server to compile it.
const FIRST_PAINT = { timeout: 20_000 };

const CHIP_ON = '.model-chips button[aria-pressed="true"] .chip-name';

test.describe('/comparer', () => {
  // SOB-224 audit: `models=` is applied, then 4 default models are added on top.
  test('quirk — ?models= with 3 models opens 7, because the 4 defaults are added', async ({
    page
  }) => {
    await page.goto('/comparer?models=gpt-5-5,o3,grok-4');
    await expect(page.getByText('7 / 8')).toBeVisible(FIRST_PAINT);

    expect((await page.locator(CHIP_ON).allInnerTexts()).sort()).toEqual([
      'Claude 3.5 Sonnet',
      'GPT-4o mini',
      'GPT-5.5',
      'Grok 4',
      'Llama 3.1 70B',
      'Mistral Large 2',
      'OpenAI o3'
    ]);
  });

  test('quirk — ?models= with 4 models opens 8, the maximum, with the 4 defaults', async ({
    page
  }) => {
    await page.goto('/comparer?models=gpt-5-5,o3,grok-4,deepseek-r1');
    await expect(page.getByText('8 / 8')).toBeVisible(FIRST_PAINT);
    expect((await page.locator(CHIP_ON).allInnerTexts()).sort()).toEqual([
      'Claude 3.5 Sonnet',
      'DeepSeek R1',
      'GPT-4o mini',
      'GPT-5.5',
      'Grok 4',
      'Llama 3.1 70B',
      'Mistral Large 2',
      'OpenAI o3'
    ]);
  });

  test('?models= sharing a model with the defaults adds only the missing ones', async ({
    page
  }) => {
    await page.goto('/comparer?models=gpt-4o-mini,o3');
    await expect(page.getByText('5 / 8')).toBeVisible(FIRST_PAINT);
    expect((await page.locator(CHIP_ON).allInnerTexts()).sort()).toEqual([
      'Claude 3.5 Sonnet',
      'GPT-4o mini',
      'Llama 3.1 70B',
      'Mistral Large 2',
      'OpenAI o3'
    ]);
  });

  test('without ?models= the 4 defaults are selected', async ({ page }) => {
    await page.goto('/comparer');
    await expect(page.getByText('4 / 8')).toBeVisible(FIRST_PAINT);
    expect((await page.locator(CHIP_ON).allInnerTexts()).sort()).toEqual([
      'Claude 3.5 Sonnet',
      'GPT-4o mini',
      'Llama 3.1 70B',
      'Mistral Large 2'
    ]);
  });
});

test.describe('/modeles', () => {
  const FIRST_EIGHT = [
    'Claude Haiku 4.5',
    'GPT-5.5',
    'Claude 3.7 Sonnet',
    'Claude Sonnet 4.6',
    'Claude Sonnet 4',
    'Claude Opus 4',
    'Claude Opus 4.8',
    'Claude Opus 4.7'
  ];

  // SOB-224 audit: only the first 8 models of the catalogue get a baseline.
  test('quirk — the CO₂ ascending sort ranks 8 models, then the other 26 by name', async ({
    page
  }) => {
    await page.goto('/modeles');
    await expect(page.getByRole('grid')).toHaveAttribute('aria-label', /34 visibles/, FIRST_PAINT);
    await page.getByLabel('Critère de tri').selectOption('co2eq_asc');

    const names = await page.locator('h3.model-name').allInnerTexts();
    expect(names).toHaveLength(34);
    expect(names.slice(0, 8)).toEqual(FIRST_EIGHT);
    const rest = names.slice(8);
    expect(rest).toHaveLength(26);
    expect(rest).toEqual([...rest].sort((a, b) => a.localeCompare(b, 'fr')));
  });

  test('quirk — ?id= and ?focus= are ignored: the full catalogue shows, no drawer opens', async ({
    page
  }) => {
    for (const query of ['id=gpt-4o', 'focus=claude-haiku-4-5']) {
      await page.goto(`/modeles?${query}`);
      await expect(page.getByRole('grid')).toHaveAttribute(
        'aria-label',
        /34 visibles/,
        FIRST_PAINT
      );
      await expect(page.getByRole('dialog')).toHaveCount(0);
    }
  });
});

test.describe('home /', () => {
  test('the top bar says 100 % local, no data sent', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.local-pill')).toHaveText(/100 % local · aucune donnée envoyée/);
  });

  // SOB-224 audit: nothing is logged in demo mode, but the page says it is.
  test('quirk — the demo says the estimate is logged in the audit ledger, entry #0, chain intact', async ({
    page
  }) => {
    await page.goto('/');
    const signature = await estimateFromHome(
      page,
      'Explique-moi la photosynthèse en trois phrases.'
    );

    await expect(signature).toContainText("Estimation journalisée dans le ledger d'audit · entrée");
    await expect(signature.locator('a.hash')).toHaveText(/#0 · seed 42/);
    await expect(signature.locator('a.hash')).toHaveAttribute('href', '/journal?focus=0');
    await expect(signature).toContainText('Chaîne intègre');
  });

  test('the "Voir aussi" panel is not shown in demo mode: the preferences are not loaded', async ({
    page
  }) => {
    await page.goto('/');
    await estimateFromHome(page, 'Bonjour');

    await expect(page.getByText('Voir aussi (autres méthodologies)')).toHaveCount(0);
  });
});

test.describe('"100 % local" claims', () => {
  // The home page calls the same comparison estimates "not logged" (ephemeral).
  test('quirk — /comparer says its values are logged in the local ledger', async ({ page }) => {
    await page.goto('/comparer');
    await expect(page.getByText('Calculs 100 % locaux')).toBeVisible();

    await page.getByRole('button', { name: 'Comparer', exact: true }).click();
    const footnote = page.locator('p.cards-footnote');
    await expect(footnote).toBeVisible({ timeout: 15_000 });
    await expect(footnote).toContainText(
      /10\s000\s+tirages\s+par\s+mod\S*le,\s+journalis\S*es\s+dans\s+le\s+ledger\s+d.audit\s+local\./
    );
  });

  test('/methodologies and /methodo say they are 100 % local', async ({ page }) => {
    await page.goto('/methodologies');
    await expect(page.locator('.local-pill')).toHaveText(/100 % local/);

    await page.goto('/methodo');
    await expect(page.getByText('Doc 100 % locale')).toBeVisible();
  });
});
