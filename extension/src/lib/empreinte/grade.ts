// Sobr.ia extension — score A-F + ratio gauge (design 38).
//
// Mirror simplifié de l'échelle score A-F du design system Sobr.ia
// (cf. preview/05-score.html et tokens --score-a..--score-f).
//
// Convention : sept paliers d'empreinte par requête (g CO₂eq).
// Calibration approximative basée sur EcoLogits 2026-01 :
//   A < 0.5  · B < 1.5  · C < 3  · D < 6  · E < 12  · F ≥ 12
// (réajuster en C27.6 selon les retours de la candidature data.gouv.fr.)

export type Grade = 'A' | 'B' | 'C' | 'D' | 'E' | 'F';

const THRESHOLDS: ReadonlyArray<readonly [number, Grade]> = [
  [0.5, 'A'],
  [1.5, 'B'],
  [3.0, 'C'],
  [6.0, 'D'],
  [12.0, 'E']
];

/** Note Sobr.ia (A meilleur ↔ F pire) en fonction du g CO₂eq d'une requête. */
export function pickGrade(gco2eq: number): Grade {
  for (const [max, grade] of THRESHOLDS) {
    if (gco2eq < max) return grade;
  }
  return 'F';
}

/** Ton couleur selon la note A-F (popup, composer et badge in-page). */
export function toneOf(gco2eq: number): 'lime' | 'amber' | 'coral' {
  const g = pickGrade(gco2eq);
  if (g === 'A' || g === 'B') return 'lime';
  if (g === 'C' || g === 'D') return 'amber';
  return 'coral';
}

/**
 * Ratio circonférentiel de la jauge (0 = vide, 1 = plein).
 *
 * Calé sur l'échelle log de 0 à 12 g (F). Au-delà de 12 g la jauge est
 * pleine pour ne pas saturer visuellement.
 */
export function gaugeRatio(gco2eq: number): number {
  const max = 12.0;
  if (gco2eq <= 0) return 0;
  if (gco2eq >= max) return 1;
  // Mapping log doux : note A reste visible, F arrive vite.
  return Math.min(1, Math.log10(1 + gco2eq * 9) / Math.log10(1 + max * 9));
}
