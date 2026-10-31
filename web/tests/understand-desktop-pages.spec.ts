import { expect, test, type Page } from '@playwright/test';

import {
  DATASHEET,
  industrialSites,
  ONBOARDED_PREFS,
  PROJECT,
  regionAggregates,
  SANKEY
} from './helpers/fixtures';
import { settled } from './helpers/settle';
import { installTauriStub, ipcCalls } from './helpers/tauri-stub';

/**
 * `/territoire` and `/datasheets` on the desktop path (SOB-224), through the IPC stub: no
 * Playwright run reaches these paths in demo mode. Tests named `quirk` pin a defect as it is.
 */

// The map loads Leaflet on first view, which can wait for the dev server.
const MAP_READY = { timeout: 20_000 };

test.describe('/territoire', () => {
  const SITES = industrialSites(200);

  test.beforeEach(async ({ page }) => {
    // The map tiles come from a CDN (a-d.basemaps.cartocdn.com): the specs run without network.
    await page.route(/\.basemaps\.cartocdn\.com\//, (route) => route.abort());
  });

  async function openWith(page: Page, sites: typeof SITES) {
    await installTauriStub(page, {
      responses: {
        list_industrial_sites_fr: sites,
        aggregate_industrial_sites_by_region: regionAggregates(sites),
        sankey_fr_data: SANKEY
      },
      state: { prefs: ONBOARDED_PREFS }
    });
    await page.goto('/territoire');
  }

  test('loads the top 200 sites once, with the map, the filters and the Sankey', async ({
    page
  }) => {
    await openWith(page, SITES);

    await expect(page.locator('.leaflet-container')).toBeAttached(MAP_READY);
    await expect(page.locator('.sankey-card')).toBeVisible();
    await settled(page);
    await expect(page.getByRole('alert')).toHaveCount(0);
    const calls = await ipcCalls(page, 'list_industrial_sites_fr');
    expect(calls.map((c) => c.args)).toEqual([{ limit: 200, offset: 0 }]);
  });

  // SOB-224 audit: no paging and no notice that only the top 200 are loaded.
  test('quirk — the sites are not paged: one call of 200, and no notice about the rest', async ({
    page
  }) => {
    await openWith(page, SITES);
    await expect(page.locator('.leaflet-container')).toBeAttached(MAP_READY);

    await settled(page);
    expect(await ipcCalls(page, 'list_industrial_sites_fr')).toHaveLength(1);
    await expect(page.locator('section.hero')).toHaveText(
      "Cartographie IRIS · 13 régions L'angle territorial français de l'impact IA. 200 sites industriels FR top-consommateurs (RTE IRIS / ODRÉ), agrégés par région avec leur mix énergétique. Sankey national en bas de page pour situer l'usage IA dans la production électrique réelle 2023."
    );
    await expect(page.locator('div.grid')).toHaveText(
      "Filtres Restreindre la carte Régions toutes IDF Île-de-France 67 ARA Auvergne-Rhône-Alpes 67 HDF Hauts-de-France 66 Conso. ≥ 0 GWh 0 201 GWh +− Leaflet | © OpenStreetMap · © CARTO Zoom 5/18 · vue régionale (13 régions) Cliquez un marker Sites individuels au zoom ≥ 6, agrégats régionaux en dessous. Le détail de la sélection s'affiche ici."
    );
  });

  test('quirk — the hero says 200 sites whatever the backend returned', async ({ page }) => {
    await openWith(page, industrialSites(3));

    await expect(page.locator('.leaflet-container')).toBeAttached(MAP_READY);
    await expect(page.getByText(/200 sites industriels FR top-consommateurs/)).toBeVisible();
  });
});

test.describe('/datasheets', () => {
  test.use({ acceptDownloads: false });

  test.beforeEach(async ({ page }) => {
    await installTauriStub(page, {
      responses: { list_projects: [PROJECT], generate_project_datasheet: DATASHEET },
      state: { prefs: ONBOARDED_PREFS }
    });
    await page.goto('/datasheets');
    await page.getByRole('button', { name: `Voir le datasheet de ${PROJECT.name}` }).click();
    await expect(page.getByRole('button', { name: 'Télécharger en .jsonld' })).toBeVisible();
  });

  test('opening a project asks the backend for its datasheet', async ({ page }) => {
    const calls = await ipcCalls(page, 'generate_project_datasheet');

    expect(calls.map((c) => c.args)).toEqual([{ id: PROJECT.id }]);
  });

  // SOB-224 audit: the save goes through `<a download>` and cannot know if a file was written.
  test('quirk — the save reports "Fichier écrit" even when the browser cancels the download', async ({
    page
  }) => {
    const download = page.waitForEvent('download');
    await page.getByRole('button', { name: 'Télécharger en .jsonld' }).click();

    await expect(
      page.getByText('Fichier écrit : etude-pilote-ete-2026-datasheet.jsonld')
    ).toBeVisible();
    await expect(page.getByRole('button', { name: 'Enregistré' })).toBeVisible();
    // `acceptDownloads: false` makes the browser cancel the download, as a webview could.
    await expect((await download).failure()).resolves.not.toBeNull();
  });
});
