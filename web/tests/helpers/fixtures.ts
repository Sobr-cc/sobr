/**
 * Small hand-written fixtures for the IPC stub, shaped like the DTOs of `src/lib/api.ts`.
 * They only feed the desktop code paths of the pages; the numbers mean nothing.
 */

const REGIONS = [
  { iso: 'FR-IDF', name: 'Île-de-France', insee: '11', lat: 48.7, lon: 2.6, nuclear: 0 },
  { iso: 'FR-ARA', name: 'Auvergne-Rhône-Alpes', insee: '84', lat: 45.5, lon: 4.8, nuclear: 74.5 },
  { iso: 'FR-HDF', name: 'Hauts-de-France', insee: '32', lat: 50.0, lon: 2.7, nuclear: 78.4 }
];

export function industrialSites(count: number) {
  return Array.from({ length: count }, (_, i) => {
    const region = REGIONS[i % REGIONS.length] as (typeof REGIONS)[number];
    return {
      code_iris: `${100000000 + i}`,
      commune: `Commune ${i + 1}`,
      department_code: `${(i % 90) + 1}`.padStart(2, '0'),
      region_iso: region.iso,
      lat: region.lat + (i % 7) * 0.05,
      lon: region.lon + (i % 5) * 0.05,
      consumption_mwh_elec: 1000 * (count - i),
      consumption_mwh_gas: 100,
      consumption_total_mwh: 1000 * (count - i) + 100,
      pdl_total: 3,
      year: 0
    };
  });
}

export function regionAggregates(sites: ReturnType<typeof industrialSites>) {
  return REGIONS.map((region) => {
    const own = sites.filter((s) => s.region_iso === region.iso);
    return {
      region_iso: region.iso,
      region_name: region.name,
      insee_code: region.insee,
      site_count: own.length,
      total_consumption_mwh_elec: own.reduce((sum, s) => sum + s.consumption_mwh_elec, 0),
      total_consumption_mwh_gas: own.reduce((sum, s) => sum + s.consumption_mwh_gas, 0),
      total_consumption_mwh: own.reduce((sum, s) => sum + s.consumption_total_mwh, 0),
      centroid_lat: region.lat,
      centroid_lon: region.lon,
      nuclear_share_pct: region.nuclear,
      top_sites: own.slice(0, 5).map((s) => ({
        code_iris: s.code_iris,
        commune: s.commune,
        consumption_total_mwh: s.consumption_total_mwh
      }))
    };
  });
}

export const SANKEY = {
  nodes: [
    { id: 'prod-nuclear', label: 'Nucléaire', layer: 0, value_twh: 300 },
    { id: 'prod-hydro', label: 'Hydraulique', layer: 0, value_twh: 60 },
    { id: 'use-domestic', label: 'Consommation intérieure', layer: 1, value_twh: 360 }
  ],
  links: [
    { source: 'prod-nuclear', target: 'use-domestic', value_twh: 300 },
    { source: 'prod-hydro', target: 'use-domestic', value_twh: 60 }
  ],
  total_production_twh: 360,
  year: 2023,
  source_url: 'https://example.org/eco2mix',
  source_sha256: 'deadbeef00000000000000000000000000000000000000000000000000000000'
};

export const PROJECT = {
  id: 1,
  name: 'Étude pilote Été 2026',
  description: 'Projet de test',
  period_start: '2026-05-01T00:00:00Z',
  period_end: '2026-05-31T23:59:59Z',
  tags: ['test'],
  created_at: '2026-05-01T08:00:00Z',
  updated_at: '2026-05-02T08:00:00Z'
};

export const DATASHEET = {
  project: PROJECT,
  jsonld: {
    '@context': { '@vocab': 'https://schema.org/' },
    '@graph': [{ '@id': '#project-1', '@type': 'Dataset', name: PROJECT.name }]
  },
  composition: {
    total_requests: 12,
    unique_models: ['gpt-4o-mini'],
    total_co2eq_g_p50: 25.5,
    total_energy_wh_p50: 80.25,
    total_water_l_p50: 0.4,
    date_first_entry: '2026-05-02T09:00:00Z',
    date_last_entry: '2026-05-30T17:00:00Z'
  },
  sha256: 'deadbeef00000000000000000000000000000000000000000000000000000000'
};
