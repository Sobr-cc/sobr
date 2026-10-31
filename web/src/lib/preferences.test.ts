import { get } from 'svelte/store';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { AppPreferencesDto, ModuleId, Persona } from './api';

vi.mock('./api', () => ({
  getAppPreferences: vi.fn(),
  setAppPreferences: vi.fn()
}));

import {
  ALL_MODULES,
  ALL_PERSONAS,
  CATEGORY_LABELS,
  defaultModulesFor,
  moduleCategory,
  moduleDescription,
  moduleHref,
  moduleLabel,
  moduleReason,
  personaLabel,
  personaTagline
} from './preferences';

async function fresh() {
  vi.resetModules();
  const api = vi.mocked(await import('./api'));
  const store = await import('./preferences');
  return { api, ...store };
}

function deferred() {
  let resolve!: () => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<void>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const SAVED: AppPreferencesDto = {
  persona: 'researcher',
  enabled_modules: ['m1', 'm3'],
  onboarded: true,
  lang: 'en',
  default_method: 'ecologits',
  also_show_methods: ['afnor_sobria'],
  default_datacenter_id: 'dc-eu-1'
};

beforeEach(() => {
  vi.resetAllMocks();
});

describe('preferences store', () => {
  it('starts unloaded with the French and AFNOR defaults', async () => {
    const { preferences } = await fresh();

    expect(get(preferences)).toEqual({
      persona: null,
      enabled_modules: [],
      onboarded: false,
      lang: 'fr',
      default_method: 'afnor_sobria',
      also_show_methods: [],
      default_datacenter_id: undefined,
      loaded: false
    });
  });

  it('loadPreferences replaces the state with the IPC answer and marks it loaded', async () => {
    const { api, preferences, loadPreferences } = await fresh();
    api.getAppPreferences.mockResolvedValue(SAVED);

    await loadPreferences();

    expect(get(preferences)).toEqual({ ...SAVED, loaded: true });
  });

  it('loadPreferences keeps the state and rethrows when the IPC is unavailable', async () => {
    const { api, preferences, loadPreferences } = await fresh();
    const error = new Error('no ipc');
    api.getAppPreferences.mockRejectedValue(error);

    await expect(loadPreferences()).rejects.toBe(error);

    expect(get(preferences).loaded).toBe(false);
    expect(get(preferences).persona).toBeNull();
  });

  it('savePreferences shows the new state before the IPC answers, then keeps it', async () => {
    const { api, preferences, savePreferences } = await fresh();
    const pending = deferred();
    api.setAppPreferences.mockReturnValue(pending.promise);

    const saving = savePreferences(SAVED);

    expect(get(preferences)).toEqual({ ...SAVED, loaded: true });
    expect(api.setAppPreferences.mock.calls).toEqual([[SAVED]]);
    pending.resolve();
    await saving;
    expect(get(preferences)).toEqual({ ...SAVED, loaded: true });
  });

  it('savePreferences restores the previous state and rethrows when the IPC fails', async () => {
    const { api, preferences, loadPreferences, savePreferences } = await fresh();
    const before: AppPreferencesDto = { ...SAVED, persona: 'student', lang: 'fr' };
    api.getAppPreferences.mockResolvedValue(before);
    await loadPreferences();
    const error = new Error('write failed');
    api.setAppPreferences.mockRejectedValue(error);

    await expect(savePreferences(SAVED)).rejects.toBe(error);

    expect(get(preferences)).toEqual({ ...before, loaded: true });
  });

  it('savePreferences rolls back to the unloaded initial state when nothing was loaded', async () => {
    const { api, preferences, savePreferences } = await fresh();
    api.setAppPreferences.mockRejectedValue(new Error('write failed'));

    await expect(savePreferences(SAVED)).rejects.toThrow('write failed');

    expect(get(preferences).loaded).toBe(false);
    expect(get(preferences).persona).toBeNull();
    expect(get(preferences).lang).toBe('fr');
  });
});

describe('catalogues', () => {
  it('lists the five personas in order', () => {
    expect(ALL_PERSONAS).toEqual([
      'student',
      'pro_tech',
      'enterprise',
      'public_sector',
      'researcher'
    ]);
  });

  it('lists 24 modules in order, without m4', () => {
    expect(ALL_MODULES).toEqual([
      'm1',
      'm2',
      'm3',
      'm5',
      'm6',
      'm7',
      'm8',
      'm9',
      'm10',
      'm11',
      'm12',
      'm13',
      'm14',
      'm15',
      'm16',
      'm17',
      'm18',
      'm19',
      'm20',
      'm21',
      'm22',
      'm23',
      'm24',
      'm25'
    ]);
  });

  it('labels the four module categories in French', () => {
    expect(CATEGORY_LABELS).toEqual({
      estimation: 'Estimation',
      visualisation: 'Visualisation',
      reporting: 'Reporting',
      pedagogie: 'Pédagogie'
    });
  });
});

describe('defaultModulesFor', () => {
  it.each<[Persona | null, ModuleId[]]>([
    ['student', ['m1', 'm8', 'm13', 'm15', 'm25']],
    ['pro_tech', ['m1', 'm3', 'm7', 'm8', 'm9', 'm13']],
    ['enterprise', ['m1', 'm7', 'm12', 'm15', 'm17', 'm20', 'm22', 'm25']],
    ['public_sector', ['m1', 'm8', 'm12', 'm17', 'm20', 'm22']],
    ['researcher', ['m1', 'm3', 'm7', 'm8', 'm9', 'm17']],
    [null, []]
  ])('returns the bundle of %s', (persona, bundle) => {
    expect(defaultModulesFor(persona)).toEqual(bundle);
  });

  it('only returns modules of the catalogue', () => {
    for (const persona of ALL_PERSONAS) {
      for (const id of defaultModulesFor(persona)) {
        expect(ALL_MODULES).toContain(id);
      }
    }
  });

  it('returns a fresh array on each call', () => {
    const a = defaultModulesFor('student');
    a.push('m2');

    expect(defaultModulesFor('student')).toEqual(['m1', 'm8', 'm13', 'm15', 'm25']);
  });
});

describe('persona catalogue', () => {
  it.each<[Persona, string]>([
    ['student', 'Étudiant·e / Curieux·se'],
    ['pro_tech', 'Professionnel·le tech'],
    ['enterprise', 'Entreprise'],
    ['public_sector', 'Collectivité / Service public'],
    ['researcher', 'Chercheur·se / Journaliste']
  ])('labels %s as %s', (persona, label) => {
    expect(personaLabel(persona)).toBe(label);
  });

  it('keeps the taglines', () => {
    expect(Object.fromEntries(ALL_PERSONAS.map((p) => [p, personaTagline(p)]))).toMatchSnapshot();
  });
});

describe('module catalogue', () => {
  const rows: [
    ModuleId,
    string,
    'estimation' | 'visualisation' | 'reporting' | 'pedagogie',
    string | null
  ][] = [
    ['m1', 'Estimer un prompt', 'estimation', '/'],
    ['m2', 'Workbench multi-prompts', 'estimation', null],
    ['m3', 'Comparer modèles', 'estimation', '/comparer'],
    ['m5', 'Exporter rapport', 'reporting', null],
    ['m6', 'Géoloc datacenter', 'visualisation', null],
    ['m7', "Journal d'audit", 'reporting', '/journal'],
    ['m8', 'Méthodologie', 'pedagogie', '/methodo'],
    ['m9', 'Bibliothèque de modèles', 'pedagogie', '/modeles'],
    ['m10', 'Importer batch', 'estimation', null],
    ['m11', 'Extension navigateur', 'estimation', null],
    ['m12', 'Datacenters Europe', 'visualisation', '/datacenters'],
    ['m13', 'Simulateur « Et si...? »', 'estimation', '/simuler'],
    ['m14', 'À propos / Crédits', 'pedagogie', null],
    ['m15', 'Tableau de bord personnel', 'visualisation', '/suivi'],
    ['m16', 'Forecast 12 mois', 'reporting', null],
    ['m17', 'Datasheet scientifique', 'reporting', '/datasheets'],
    ['m18', 'Batch CSV → rapport', 'reporting', null],
    ['m19', 'Équipe multi-utilisateurs', 'reporting', null],
    ['m20', 'Territoire France', 'visualisation', '/territoire'],
    ['m21', 'Alertes & seuils', 'reporting', null],
    ['m22', 'Rapport réglementaire (CSRD/AGEC)', 'reporting', '/rapport-csrd'],
    ['m23', 'Marchés publics IA frugale', 'reporting', null],
    ['m24', 'Apprendre', 'pedagogie', null],
    ['m25', 'Objectifs & habitudes', 'pedagogie', null]
  ];

  it('covers exactly the modules of ALL_MODULES', () => {
    expect(rows.map(([id]) => id)).toEqual([...ALL_MODULES]);
  });

  it.each(rows)('describes %s as "%s" (%s, %s)', (id, label, category, href) => {
    expect(moduleLabel(id)).toBe(label);
    expect(moduleCategory(id)).toBe(category);
    expect(moduleHref(id)).toBe(href);
    expect(moduleDescription(id).length).toBeGreaterThan(0);
  });

  it('keeps the descriptions', () => {
    expect(
      Object.fromEntries(ALL_MODULES.map((id) => [id, moduleDescription(id)]))
    ).toMatchSnapshot();
  });
});

describe('moduleReason', () => {
  it('has no reason without a persona', () => {
    expect(moduleReason(null, 'm1')).toBeUndefined();
  });

  it('gives the persona-specific sentence for a module of its bundle', () => {
    expect(moduleReason('student', 'm1')).toBe(
      "Mesurer chaque prompt pour comprendre l'ordre de grandeur (CO₂, eau)."
    );
    expect(moduleReason('enterprise', 'm1')).toBe(
      "Référence d'estimation auditable pour ton scope 3 IA."
    );
  });

  it('has no reason for a module outside the persona bundle', () => {
    expect(moduleReason('student', 'm20')).toBeUndefined();
    expect(moduleReason('public_sector', 'm25')).toBeUndefined();
  });

  it('has a reason for every module of every bundle, and for no other', () => {
    for (const persona of ALL_PERSONAS) {
      const bundle = defaultModulesFor(persona);
      const withReason = ALL_MODULES.filter((id) => moduleReason(persona, id) !== undefined);
      expect(withReason).toEqual(bundle);
    }
  });

  it('keeps the reasons', () => {
    const table = Object.fromEntries(
      ALL_PERSONAS.map((p) => [
        p,
        Object.fromEntries(defaultModulesFor(p).map((id) => [id, moduleReason(p, id)]))
      ])
    );

    expect(table).toMatchSnapshot();
  });
});
