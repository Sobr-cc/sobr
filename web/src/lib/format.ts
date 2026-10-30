// Formatage numérique partagé (fr-FR). Deux familles :
//
//  - **valeurs par requête** (souvent 10⁻⁵..10⁻⁷ g) : chiffres significatifs
//    + bascule vers la sous-unité SI où la valeur tombe dans [1, 1000)
//    (`fmtSig`, `pickScale`) — un `maximumFractionDigits: 2` afficherait
//    « 0,00 » pour tout ce qui passe sous 5 mg ;
//  - **totaux agrégés** (suivi, datasheets, rapport CSRD) : décimales fixes
//    + sur-unités kg/t, kWh/MWh, m³ (`fmtFixed`, `fmtTotal`).

/** Nombre à `sig` chiffres significatifs ; « — » si non fini, « 0 » si nul. */
export function fmtSig(value: number, sig = 3): string {
  if (!Number.isFinite(value)) return '—';
  if (value === 0) return '0';
  return new Intl.NumberFormat('fr-FR', {
    maximumSignificantDigits: sig,
    minimumSignificantDigits: 1
  }).format(value);
}

/** Nombre à `digits` décimales au plus ; « — » si non fini. */
export function fmtFixed(value: number, digits = 2): string {
  if (!Number.isFinite(value)) return '—';
  return new Intl.NumberFormat('fr-FR', { maximumFractionDigits: digits }).format(value);
}

export type Scale = { mult: number; unit: string };

/**
 * Chaînes de préfixes SI par unité canonique renvoyée par le Rust
 * (`gCO2eq`, `Wh`, `L`, `mg`), de la plus grande à la plus petite.
 */
export const UNIT_CHAINS: Readonly<Record<string, readonly Scale[]>> = {
  gCO2eq: [
    { mult: 1e-3, unit: 'kg CO₂eq' },
    { mult: 1, unit: 'g CO₂eq' },
    { mult: 1e3, unit: 'mg CO₂eq' },
    { mult: 1e6, unit: 'µg CO₂eq' },
    { mult: 1e9, unit: 'ng CO₂eq' }
  ],
  Wh: [
    { mult: 1e-6, unit: 'MWh' },
    { mult: 1e-3, unit: 'kWh' },
    { mult: 1, unit: 'Wh' },
    { mult: 1e3, unit: 'mWh' },
    { mult: 1e6, unit: 'µWh' },
    { mult: 1e9, unit: 'nWh' }
  ],
  L: [
    { mult: 1e-3, unit: 'm³' },
    { mult: 1, unit: 'L' },
    { mult: 1e3, unit: 'mL' },
    { mult: 1e6, unit: 'µL' },
    { mult: 1e9, unit: 'nL' }
  ],
  mg: [
    { mult: 1e-3, unit: 'g' },
    { mult: 1, unit: 'mg' },
    { mult: 1e3, unit: 'µg' },
    { mult: 1e6, unit: 'ng' },
    { mult: 1e9, unit: 'pg' }
  ],
  EUR: [{ mult: 1, unit: '€' }]
};

/**
 * Première échelle de la chaîne où `value × mult` tombe dans [1, 1000).
 * À choisir une fois (sur le P50, ou la médiane d'une colonne) puis à
 * appliquer aux autres valeurs pour qu'elles restent comparables.
 */
export function pickScale(value: number, baseUnit: string): Scale {
  const chain = UNIT_CHAINS[baseUnit];
  const fallback: Scale = { mult: 1, unit: baseUnit };
  if (!chain || chain.length === 0) return fallback;
  if (!Number.isFinite(value) || value === 0) {
    // Pas de signal → unité canonique du milieu de chaîne.
    return chain.find((s) => s.mult === 1) ?? chain[0] ?? fallback;
  }
  for (const s of chain) {
    const scaled = Math.abs(value * s.mult);
    if (scaled >= 1 && scaled < 1000) return s;
  }
  // Hors plage couverte : l'extrême le plus fin, pour ne pas afficher zéro.
  return chain[chain.length - 1] ?? fallback;
}

export type AutoScale = { v: string; u: string };

/** Total agrégé avec sur-unité (g → kg → t, Wh → kWh → MWh, mL ← L → m³). */
export function fmtTotal(value: number, base: 'gCO2eq' | 'Wh' | 'L'): AutoScale {
  if (base === 'gCO2eq') {
    if (!Number.isFinite(value)) return { v: '—', u: 'g CO₂eq' };
    if (value >= 1e6) return { v: fmtFixed(value / 1e6), u: 't CO₂eq' };
    if (value >= 1e3) return { v: fmtFixed(value / 1e3), u: 'kg CO₂eq' };
    return { v: fmtFixed(value), u: 'g CO₂eq' };
  }
  if (base === 'Wh') {
    if (!Number.isFinite(value)) return { v: '—', u: 'Wh' };
    if (value >= 1e6) return { v: fmtFixed(value / 1e6), u: 'MWh' };
    if (value >= 1e3) return { v: fmtFixed(value / 1e3), u: 'kWh' };
    return { v: fmtFixed(value), u: 'Wh' };
  }
  if (!Number.isFinite(value)) return { v: '—', u: 'L' };
  if (value >= 1000) return { v: fmtFixed(value / 1000), u: 'm³' };
  if (value >= 1) return { v: fmtFixed(value), u: 'L' };
  return { v: fmtFixed(value * 1000, 1), u: 'mL' };
}

/** Entier formaté (séparateur de milliers fr-FR) ; « — » si non fini. */
export function fmtInt(value: number): string {
  return fmtFixed(value, 0);
}
