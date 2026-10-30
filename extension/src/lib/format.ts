// Sobr.ia extension — formatage partagé (popup, badge in-page, composer).

/** Formatage FR à `digits` chiffres significatifs (« 12 », « 1,8 »). */
export function fmtFr(n: number, digits = 2): string {
  return new Intl.NumberFormat('fr-FR', { maximumSignificantDigits: digits }).format(n);
}
