import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type {
  AppPreferencesDto,
  CountryAggregateDto,
  DailySeriesPointDto,
  DashboardSummaryDto,
  EstimationResultDto,
  ModelDetailDto,
  TripletDto
} from '../api';

type Handlers = (typeof import('./index'))['demoHandlers'];

// SOB-183: the demo returns these triplets with undefined values.
type Triplet = { [K in keyof TripletDto]: TripletDto[K] | undefined };
type ModelDetail = Omit<
  ModelDetailDto,
  'epsilon_prefill_mj_per_token' | 'epsilon_decode_mj_per_token' | 'embodied_g_per_request'
> & {
  epsilon_prefill_mj_per_token: Triplet;
  epsilon_decode_mj_per_token: Triplet;
  embodied_g_per_request: Triplet;
};
type Dashboard = Omit<DashboardSummaryDto, 'vs_previous'> & {
  vs_previous: NonNullable<DashboardSummaryDto['vs_previous']>;
};
interface Thrown {
  code: string;
  message: string;
}

const NOW = new Date(Date.UTC(2026, 4, 14, 10, 0, 0));

let handlers: Handlers;

function call<T = unknown>(name: string, args?: Record<string, unknown>): T {
  const handler = handlers[name];
  if (handler === undefined) throw new Error(`no demo handler named ${name}`);
  return handler(args) as T;
}

function thrown(fn: () => unknown): Thrown {
  try {
    fn();
  } catch (e) {
    return e as Thrown;
  }
  throw new Error('expected the handler to throw');
}

beforeEach(async () => {
  vi.resetModules();
  handlers = (await import('./index')).demoHandlers;
});

describe('command table', () => {
  it('serves exactly the commands the web demo covers', () => {
    expect(Object.keys(handlers).sort()).toEqual([
      'aggregate_datacenters_by_country',
      'estimate_for_comparison',
      'estimate_prompt',
      'get_app_preferences',
      'get_dashboard_summary',
      'get_datacenter_detail',
      'get_model_detail',
      'get_referentiel_status',
      'list_datacenters',
      'list_methodologies',
      'list_models',
      'list_vendor_comparison',
      'meta_info',
      'set_app_preferences'
    ]);
  });
});

describe('static information', () => {
  it('describes the demo build', () => {
    expect(call('meta_info')).toEqual({
      app_version: 'démo web (fixtures seed 42)',
      estimator_seed: 42,
      estimator_n: 10_000,
      audit_path: '— (ledger local : application de bureau)',
      data_root: '— (référentiel Gold : application de bureau)'
    });
  });

  it('reports the Gold referential as unavailable with the model count', () => {
    expect(call('get_referentiel_status')).toEqual({
      available: false,
      message:
        'Mode démo web — le référentiel Gold (pipeline médaillon Copper→Silver→Gold) ' +
        "est embarqué dans l'application de bureau.",
      version: '',
      snapshot_date: '',
      sha256: '',
      source_count: 0,
      model_count: 34,
      path: ''
    });
  });

  it('lists the two methodologies', () => {
    const list = call<{ method: string; display_name: string }[]>('list_methodologies');

    expect(list.map((m) => [m.method, m.display_name])).toEqual([
      ['afnor_sobria', 'AFNOR SPEC 2314 — Sobr.ia'],
      ['ecologits', 'EcoLogits 2026-01']
    ]);
  });
});

describe('list_models', () => {
  it('returns the 34 precomputed models with the preset fields', () => {
    const models = call<Record<string, unknown>[]>('list_models');

    expect(models).toHaveLength(34);
    expect(models[0]).toStrictEqual({
      id: 'claude-opus-4-8',
      display_name: 'Claude Opus 4.8',
      provider: 'Anthropic',
      family: 'claude-4',
      approx_params_billions: 2000,
      openness: 'closed',
      calibration: 'extrapolated',
      sources: [
        'Anthropic Claude Opus 4.8 release (2026-05)',
        'Estimation taille — analyse publique 2026'
      ],
      release_date: '2026-05-15',
      active_params_b: 2000,
      model_family: 'anthropic',
      architecture: 'dense_transformer',
      vision_capable: true,
      audio_capable: false,
      reasoning_capable: true,
      thinking_token_multiplier: [2, 50],
      default_context_overhead_tokens: 2000,
      deprecated: false,
      source_url: 'https://www.anthropic.com/news'
    });
    expect(models.at(-1)?.id).toBe('gemini-2-0-flash');
  });

  it('covers nine providers, flattens the architecture and omits an absent thinking multiplier', () => {
    const models = call<Record<string, unknown>[]>('list_models');

    expect([...new Set(models.map((m) => m.provider))]).toEqual([
      'Anthropic',
      'OpenAI',
      'Google',
      'Meta',
      'Mistral AI',
      'DeepSeek',
      'xAI',
      'Alibaba',
      'Microsoft'
    ]);
    expect([...new Set(models.map((m) => m.architecture))].sort()).toEqual([
      'dense_transformer',
      'moe'
    ]);
    expect(models.filter((m) => 'thinking_token_multiplier' in m)).toHaveLength(21);
    expect(models.find((m) => m.id === 'gpt-5-5')).not.toHaveProperty('thinking_token_multiplier');
  });
});

describe('list_vendor_comparison', () => {
  it('aggregates the disclosures per vendor, sorted by name', () => {
    expect(call('list_vendor_comparison')).toEqual([
      {
        vendor: 'Google',
        has_prompt_level: true,
        has_training: false,
        primary_source_url:
          'https://services.google.com/fh/files/misc/measuring_the_environmental_impact_of_delivering_ai_at_google_scale.pdf'
      },
      {
        vendor: 'Meta',
        has_prompt_level: false,
        has_training: true,
        primary_source_url:
          'https://github.com/meta-llama/llama-models/blob/main/models/llama3_1/MODEL_CARD.md'
      },
      {
        vendor: 'Mistral AI',
        has_prompt_level: true,
        has_training: true,
        primary_source_url:
          'https://mistral.ai/news/our-contribution-to-a-global-environmental-standard-for-ai'
      }
    ]);
  });
});

describe('estimate_prompt and estimate_for_comparison', () => {
  const req = (tokens_in: number, tokens_out_estimated: number, extra = {}) => ({
    req: { model_id: 'o3', tokens_in, tokens_out_estimated, ...extra }
  });

  it('serves the fixture of the grid point as a DTO with the demo hypothesis added', () => {
    const r = call<EstimationResultDto>('estimate_prompt', req(1200, 800));

    expect(r.method).toBe('afnor_sobria');
    expect(r.request).toEqual({
      model_id: 'o3',
      timestamp: '2026-06-12T00:00:00Z',
      tokens_in: 1200,
      tokens_out_estimated: 800
    });
    expect(r.seed).toBe(42);
    expect(r.audit_id).toBe(0);
    expect(r.computed_at).toBe('2026-06-12T07:12:58.236119041Z');
    expect(r.indicators.map((i) => [i.indicator, i.p5, i.p50, i.p95, i.unit, 'bins' in i])).toEqual(
      [
        ['co2eq', 1.4224811766092396, 2.2655111942652404, 3.6533325157905225, 'gCO2eq', true],
        ['energy', 23.589411130875185, 38.55973538599015, 63.279331854213396, 'Wh', true],
        ['water', 0.020797630541994745, 0.05536958391472868, 0.11683143900235834, 'L', true]
      ]
    );
    expect(r.equivalents).toHaveLength(3);
    expect(r.hypotheses.map((h) => h.key)).toEqual([
      'epsilon_prefill_mj_per_token',
      'epsilon_decode_mj_per_token',
      'pue',
      'if_electrical_g_per_kwh',
      'embodied_g_per_request',
      'wue_l_per_kwh',
      'mode_demo'
    ]);
    expect(r.hypotheses.at(-1)).toEqual({
      key: 'mode_demo',
      value:
        'Résultat précalculé par sobria-estimator (seed 42, N=10 000) au point de grille le plus ' +
        "proche de votre requête. Installez l'application pour des estimations exactes sur vos " +
        'propres prompts.',
      source: 'C36 — mode démo web, fixtures générées par le moteur Rust'
    });
  });

  it('picks the nearest grid point in log distance of both token counts', () => {
    const grid = (a: number, b: number) => {
      const r = call<EstimationResultDto>('estimate_prompt', req(a, b));
      return [r.request.tokens_in, r.request.tokens_out_estimated];
    };

    expect(grid(100, 100)).toEqual([300, 150]);
    expect(grid(1100, 700)).toEqual([1200, 800]);
    expect(grid(3500, 1500)).toEqual([4000, 2000]);
    expect(grid(1_000_000, 1)).toEqual([4000, 2000]);
  });

  it('uses the demo default method, and the request method when given', () => {
    expect(call<EstimationResultDto>('estimate_prompt', req(1200, 800)).method).toBe(
      'afnor_sobria'
    );
    expect(
      call<EstimationResultDto>('estimate_prompt', req(1200, 800, { method: 'ecologits' })).method
    ).toBe('ecologits');
  });

  it('follows the default method saved with set_app_preferences', () => {
    call('set_app_preferences', { prefs: { default_method: 'ecologits' } });

    expect(call<EstimationResultDto>('estimate_prompt', req(1200, 800)).method).toBe('ecologits');
    expect(
      call<EstimationResultDto>('estimate_prompt', req(1200, 800, { method: 'afnor_sobria' }))
        .method
    ).toBe('afnor_sobria');
  });

  it('estimate_for_comparison takes the method from its own argument', () => {
    const r = call<EstimationResultDto>('estimate_for_comparison', {
      ...req(1200, 800),
      method: 'ecologits'
    });

    expect(r.method).toBe('ecologits');
    expect(r.audit_id).toBe(0);
  });

  it('rejects a model without fixtures and lists the covered ones', () => {
    const e = thrown(() =>
      call<EstimationResultDto>('estimate_prompt', {
        req: { model_id: 'zz', tokens_in: 1, tokens_out_estimated: 1 }
      })
    );

    expect(e.code).toBe('unknown_model');
    // SOB-204: the catalogue size is hardcoded and stale (34 models in the registry).
    expect(e.message).toMatch(
      /^La démo couvre 34 modèles précalculés \(claude-opus-4-8, claude-opus-4-7, .*, gemini-2-0-flash\)\. Installez l'application pour estimer les 25 modèles du catalogue\.$/
    );
  });
});

describe('get_model_detail', () => {
  it('returns the preset, the baseline of the 1200/800 grid point and the vendor disclosures', () => {
    const d = call<ModelDetail>('get_model_detail', { id: 'o3' });

    expect(d.id).toBe('o3');
    expect(d.architecture).toBe('dense_transformer');
    expect(d.baseline_co2eq_p5_g).toBe(1.4224811766092396);
    expect(d.baseline_co2eq_p50_g).toBe(2.2655111942652404);
    expect(d.baseline_co2eq_p95_g).toBe(3.6533325157905225);
    expect(d.baseline_energy_wh_p50).toBe(38.55973538599015);
    expect(d.baseline_water_l_p50).toBe(0.05536958391472868);
    expect(d.vendor_disclosures).toEqual([]);
    expect(
      call<ModelDetail>('get_model_detail', { id: 'mistral-large-2' }).vendor_disclosures
    ).toHaveLength(3);
  });

  it('keeps the thinking multiplier only for models that have one', () => {
    expect(call<ModelDetail>('get_model_detail', { id: 'o3' }).thinking_token_multiplier).toEqual([
      5, 30
    ]);
    expect(call('get_model_detail', { id: 'gpt-4o' })).not.toHaveProperty(
      'thinking_token_multiplier'
    );
  });

  // SOB-183: the fixtures store the triplets as arrays, the adapter reads objects.
  it('returns the three distribution triplets with undefined values', () => {
    const d = call<ModelDetail>('get_model_detail', { id: 'o3' });
    const empty = { p5: undefined, p50: undefined, p95: undefined };

    expect(d.epsilon_prefill_mj_per_token).toStrictEqual(empty);
    expect(d.epsilon_decode_mj_per_token).toStrictEqual(empty);
    expect(d.embodied_g_per_request).toStrictEqual(empty);
  });

  it('asks for the desktop app for an unknown model', () => {
    const e = thrown(() => call('get_model_detail', { id: 'nope' }));

    expect(e.code).toBe('tauri_unavailable');
    expect(e.message).toMatch(
      /^La fiche distributionnelle complète de ce modèle nécessite l'application de bureau Sobr\.ia /
    );
  });
});

describe('datacenters', () => {
  it('lists the 28 embedded datacenters', () => {
    const list = call<Record<string, unknown>[]>('list_datacenters');

    expect(list).toHaveLength(28);
    expect(list[0]).toEqual({
      id: 'ovh-rbx-roubaix',
      name: 'OVH Roubaix (RBX)',
      operator: 'OVHcloud',
      country_iso: 'FR',
      city: 'Roubaix',
      lat: 50.6927,
      lon: 3.1746,
      pue: 1.09,
      wue_l_per_kwh: 0.26,
      if_electrical_g_per_kwh: 56,
      capacity_mw: 30,
      sources: [
        'https://corporate.ovhcloud.com/fr/sustainability/environment/',
        'Electricity Maps — France annual mix 2023'
      ]
    });
  });

  it('averages PUE, carbon intensity and centroid per country, in order of first appearance', () => {
    const rows = call<CountryAggregateDto[]>('aggregate_datacenters_by_country');

    expect(rows.map((r) => [r.country_iso, r.datacenter_count])).toEqual([
      ['FR', 4],
      ['DE', 4],
      ['IE', 3],
      ['NL', 3],
      ['GB', 3],
      ['SE', 2],
      ['FI', 2],
      ['ES', 2],
      ['IT', 1],
      ['PL', 1],
      ['CH', 1],
      ['AT', 1],
      ['DK', 1]
    ]);
    const fr = rows[0];
    expect(fr?.avg_pue).toBeCloseTo(1.11, 9);
    expect(fr?.if_electrical_g_per_kwh).toBe(56);
    expect(fr?.total_capacity_mw).toBe(138.5);
    expect(fr?.centroid_lat).toBeCloseTo(49.895, 9);
    expect(fr?.centroid_lon).toBeCloseTo(2.438875, 9);
  });

  it('leaves out the capacity of countries where no datacenter declares one', () => {
    const rows = call<CountryAggregateDto[]>('aggregate_datacenters_by_country');
    const withoutCapacity = rows
      .filter((r) => !('total_capacity_mw' in r))
      .map((r) => r.country_iso);

    expect(withoutCapacity).toEqual(['GB', 'ES', 'IT', 'PL']);
  });

  it('asks for the desktop app for the datacenter detail', () => {
    const e = thrown(() => call('get_datacenter_detail'));

    expect(e.code).toBe('tauri_unavailable');
    expect(e.message).toMatch(
      /^La fiche détaillée datacenter \(profil 24 h\) nécessite l'application de bureau Sobr\.ia /
    );
  });
});

describe('preferences', () => {
  const INITIAL = {
    persona: null,
    enabled_modules: [
      'm1',
      'm3',
      'm7',
      'm8',
      'm9',
      'm12',
      'm13',
      'm14',
      'm15',
      'm17',
      'm20',
      'm22',
      'm25'
    ],
    onboarded: true,
    lang: 'fr',
    default_method: 'afnor_sobria',
    also_show_methods: ['ecologits'],
    default_datacenter_id: undefined
  };

  it('starts onboarded, in French, with AFNOR as default and EcoLogits as reference', () => {
    expect(call('get_app_preferences')).toEqual(INITIAL);
  });

  it('returns a shallow copy: editing a top-level field does not change the stored preferences', () => {
    call<AppPreferencesDto>('get_app_preferences').lang = 'en';

    expect(call<AppPreferencesDto>('get_app_preferences').lang).toBe('fr');
  });

  it('shares its arrays with the stored preferences, so pushing into one changes the next answer', () => {
    call<AppPreferencesDto>('get_app_preferences').enabled_modules.push('m2');
    call<AppPreferencesDto>('get_app_preferences').also_show_methods.push('afnor_sobria');

    const next = call<AppPreferencesDto>('get_app_preferences');
    expect(next.enabled_modules).toContain('m2');
    expect(next.also_show_methods).toEqual(['ecologits', 'afnor_sobria']);
  });

  it('merges the saved fields into the in-memory preferences and returns null', () => {
    const result = call('set_app_preferences', { prefs: { lang: 'en', persona: 'student' } });

    expect(result).toBeNull();
    expect(call('get_app_preferences')).toEqual({ ...INITIAL, lang: 'en', persona: 'student' });
  });

  it('forgets the saved preferences when the module is loaded again', async () => {
    call('set_app_preferences', { prefs: { lang: 'en' } });
    vi.resetModules();
    const again = (await import('./index')).demoHandlers;

    expect((again.get_app_preferences?.() as AppPreferencesDto).lang).toBe('fr');
  });
});

describe('get_dashboard_summary', () => {
  beforeEach(() => {
    vi.stubEnv('TZ', 'UTC');
    vi.useFakeTimers();
    vi.setSystemTime(NOW);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllEnvs();
  });

  const day = (
    date: string,
    request_count: number,
    co2: number,
    energy: number,
    water: number
  ) => ({
    date,
    request_count,
    co2eq_g_p50: expect.closeTo(co2, 6),
    energy_wh_p50: expect.closeTo(energy, 6),
    water_l_p50: expect.closeTo(water, 6)
  });

  const periods = [
    {
      period: 'today',
      label: "Aujourd'hui (démo)",
      start: '2026-05-14T00:00:00.000Z',
      end: '2026-05-14T10:00:00.000Z',
      days: 1,
      requests: 266,
      co2: 328.53507713247,
      energy: 5525.20262631252,
      water: 7.929593589023544,
      previous: { co2: 387.6713910163146, requests: 293 },
      top: { model_id: 'claude-opus-4-8', request_count: 5, co2: 47.88374817914981 },
      first: day('2026-05-14', 266, 328.53507713247, 5525.20262631252, 7.929593589023544),
      lastDate: '2026-05-14'
    },
    {
      period: 'last_7_days',
      label: '7 derniers jours (démo)',
      start: '2026-05-08T00:00:00.000Z',
      end: '2026-05-14T10:00:00.000Z',
      days: 6,
      requests: 1673,
      co2: 3075.456719349295,
      energy: 51816.512026496384,
      water: 74.3493216768587,
      previous: { co2: 3629.0389288321676, requests: 1840 },
      top: { model_id: 'claude-opus-4-7', request_count: 64, co2: 612.9119766931176 },
      first: day('2026-05-08', 303, 536.1164278360327, 9026.192679043388, 12.953294633490124),
      lastDate: '2026-05-13'
    },
    {
      period: 'this_month',
      label: 'Ce mois-ci (démo)',
      start: '2026-05-01T00:00:00.000Z',
      end: '2026-05-14T10:00:00.000Z',
      days: 13,
      requests: 3684,
      co2: 6666.8846546718105,
      energy: 112452.23180446605,
      water: 161.34627779696748,
      previous: { co2: 7866.923892512736, requests: 4052 },
      top: { model_id: 'gpt-5-5-pro', request_count: 110, co2: 956.1323429896557 },
      first: day('2026-05-01', 264, 405.7201937227827, 6865.530897031909, 9.847771564622159),
      lastDate: '2026-05-13'
    },
    {
      period: 'last_month',
      label: 'Mois précédent (démo)',
      start: '2026-04-01T00:00:00.000Z',
      end: '2026-04-30T00:00:00.000Z',
      days: 29,
      requests: 8524,
      co2: 15820.356696497853,
      energy: 266641.6089218704,
      water: 382.57955032081986,
      previous: { co2: 18668.020901867465, requests: 9376 },
      top: { model_id: 'claude-opus-4-8', request_count: 263, co2: 2518.68515422328 },
      first: day('2026-04-01', 306, 595.6280731686215, 10033.792280421747, 14.39568644793272),
      lastDate: '2026-04-29'
    },
    {
      period: 'this_year',
      label: 'Cette année (démo)',
      start: '2026-01-01T00:00:00.000Z',
      end: '2026-05-14T10:00:00.000Z',
      days: 133,
      requests: 38665,
      co2: 71204.26379870113,
      energy: 1200079.606111815,
      water: 1721.8510004865925,
      previous: { co2: 84021.03128246733, requests: 42532 },
      top: { model_id: 'claude-opus-4-8', request_count: 1171, co2: 11214.37382355688 },
      first: day('2026-01-01', 296, 471.12205360804484, 7923.516905376756, 11.36977134322259),
      lastDate: '2026-05-13'
    }
  ];

  it.each(periods)('summarizes $period', (p) => {
    const s = call<Dashboard>('get_dashboard_summary', { period: p.period });

    expect(s.period_label).toBe(p.label);
    expect(s.period_start).toBe(p.start);
    expect(s.period_end).toBe(p.end);
    expect(s.daily_series).toHaveLength(p.days);
    expect(s.daily_series[0]).toEqual(p.first);
    expect(s.daily_series.at(-1)?.date).toBe(p.lastDate);
    expect(s.total_requests).toBe(p.requests);
    expect(s.total_co2eq_g_p50).toBeCloseTo(p.co2, 6);
    expect(s.total_energy_wh_p50).toBeCloseTo(p.energy, 6);
    expect(s.total_water_l_p50).toBeCloseTo(p.water, 6);
    expect(s.vs_previous.previous_total_co2eq_g_p50).toBeCloseTo(p.previous.co2, 6);
    expect(s.vs_previous.previous_total_requests).toBe(p.previous.requests);
    expect(s.vs_previous.delta_co2eq_pct).toBe(-15.3);
    expect(s.vs_previous.delta_requests_pct).toBe(-9.1);
    expect(s.top_models[0]).toEqual({
      model_id: p.top.model_id,
      request_count: p.top.request_count,
      total_co2eq_g_p50: expect.closeTo(p.top.co2, 6)
    });
    expect(s.warning_multi_method).toBe(false);
  });

  it('derives the totals from the daily series and the previous period from the totals', () => {
    const s = call<Dashboard>('get_dashboard_summary', { period: 'this_month' });
    const sum = (key: Exclude<keyof DailySeriesPointDto, 'date'>) =>
      s.daily_series.reduce((acc, d) => acc + d[key], 0);

    expect(sum('request_count')).toBe(s.total_requests);
    expect(sum('co2eq_g_p50')).toBeCloseTo(s.total_co2eq_g_p50, 6);
    expect(s.vs_previous.previous_total_co2eq_g_p50).toBeCloseTo(s.total_co2eq_g_p50 * 1.18, 9);
    expect(s.vs_previous.previous_total_requests).toBe(Math.round(s.total_requests * 1.1));
    expect(s.method_breakdown).toEqual([
      {
        method: 'afnor_sobria',
        request_count: s.total_requests,
        total_co2eq_g_p50: s.total_co2eq_g_p50,
        total_energy_wh_p50: s.total_energy_wh_p50,
        total_water_l_p50: s.total_water_l_p50
      }
    ]);
  });

  it('ranks all 34 models by CO₂ and returns the same figures on every call', () => {
    const s = call<Dashboard>('get_dashboard_summary', { period: 'last_7_days' });
    const co2 = s.top_models.map((m) => m.total_co2eq_g_p50);

    expect(s.top_models).toHaveLength(34);
    expect(co2).toEqual([...co2].sort((a: number, b: number) => b - a));
    expect(call<Dashboard>('get_dashboard_summary', { period: 'last_7_days' })).toEqual(s);
  });

  it('fails on an unknown period', () => {
    expect(() => call<Dashboard>('get_dashboard_summary', { period: 'last_decade' })).toThrow(
      TypeError
    );
  });

  // SOB-184: day labels are the UTC date of a local midnight, so they lag by one day east of UTC.
  it('labels the days one day early in Europe/Paris', () => {
    vi.stubEnv('TZ', 'Europe/Paris');

    const s = call<Dashboard>('get_dashboard_summary', { period: 'today' });

    expect(s.period_start).toBe('2026-05-13T22:00:00.000Z');
    expect(s.daily_series[0]?.date).toBe('2026-05-13');
  });

  // SOB-184: the point count is rounded from the elapsed time, so it changes at noon.
  it('returns one more daily point from noon on', () => {
    vi.setSystemTime(new Date(Date.UTC(2026, 4, 14, 14, 0, 0)));

    const week = call<Dashboard>('get_dashboard_summary', { period: 'last_7_days' });
    const month = call<Dashboard>('get_dashboard_summary', { period: 'this_month' });

    expect(week.daily_series).toHaveLength(7);
    expect(week.daily_series.at(-1)?.date).toBe('2026-05-14');
    expect(month.daily_series).toHaveLength(14);
  });
});
