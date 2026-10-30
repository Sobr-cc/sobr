import { describe, expect, it } from 'vitest';

import { fmtFixed, fmtSig, fmtTotal, pickScale } from './format';

// Intl fr-FR sépare les milliers par une espace fine insécable (U+202F).
const NNBSP = String.fromCharCode(0x202f);
const nb = (s: string) => s.split(NNBSP).join(' ');

describe('fmtSig', () => {
  it('garde 3 chiffres significatifs', () => {
    expect(fmtSig(2.14159)).toBe('2,14');
    expect(fmtSig(0.000123456)).toBe('0,000123');
  });
  it('gère zéro et non fini', () => {
    expect(fmtSig(0)).toBe('0');
    expect(fmtSig(Number.NaN)).toBe('—');
  });
});

describe('fmtFixed', () => {
  it('borne les décimales', () => {
    expect(fmtFixed(123.456)).toBe('123,46');
    expect(fmtFixed(1.25, 1)).toBe('1,3');
    expect(fmtFixed(Infinity)).toBe('—');
  });
});

describe('pickScale', () => {
  it('bascule vers la sous-unité où la valeur tombe dans [1, 1000)', () => {
    expect(pickScale(2.5e-5, 'gCO2eq').unit).toBe('µg CO₂eq');
    expect(pickScale(12, 'gCO2eq').unit).toBe('g CO₂eq');
    expect(pickScale(4500, 'Wh').unit).toBe('kWh');
    expect(pickScale(2_000_000, 'Wh').unit).toBe('MWh');
  });
  it('retombe sur l’unité canonique sans signal', () => {
    expect(pickScale(0, 'L')).toEqual({ mult: 1, unit: 'L' });
    expect(pickScale(Number.NaN, 'gCO2eq').unit).toBe('g CO₂eq');
  });
  it('prend l’extrême le plus fin hors plage, et l’unité brute si inconnue', () => {
    expect(pickScale(1e-15, 'gCO2eq').unit).toBe('ng CO₂eq');
    expect(pickScale(5, 'tokens')).toEqual({ mult: 1, unit: 'tokens' });
  });
});

describe('fmtTotal', () => {
  it('monte en kg / t pour le CO₂eq', () => {
    expect(fmtTotal(512.345, 'gCO2eq')).toEqual({ v: '512,35', u: 'g CO₂eq' });
    expect(fmtTotal(2500, 'gCO2eq')).toEqual({ v: '2,5', u: 'kg CO₂eq' });
    expect(fmtTotal(3_400_000, 'gCO2eq')).toEqual({ v: '3,4', u: 't CO₂eq' });
  });
  it('gère énergie et eau (mL sous le litre)', () => {
    expect(fmtTotal(1500, 'Wh')).toEqual({ v: '1,5', u: 'kWh' });
    expect(fmtTotal(0.25, 'L')).toEqual({ v: '250', u: 'mL' });
    expect(nb(fmtTotal(1_234_000, 'L').v)).toBe('1 234');
  });
  it('affiche un tiret si non fini', () => {
    expect(fmtTotal(Number.NaN, 'Wh')).toEqual({ v: '—', u: 'Wh' });
  });
});
