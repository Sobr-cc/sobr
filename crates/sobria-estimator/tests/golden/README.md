# Golden files de l'estimateur / Estimator golden files

## Français

Ces fichiers JSON figent le **comportement actuel** de `sobria-estimator`
(SOB-9) : valeur centrale, intervalle P5–P95, histogrammes, hypothèses,
erreurs. Ils sont lus par `tests/characterization/`.

- Entrées fixes : seed `DEFAULT_SEED` (42), horodatage des requêtes figé.
  Le champ `computed_at` (heure du calcul) est masqué (`<redacted>`).
- Comparaison : égalité exacte sur les entiers et les textes, **tolérance
  relative de 1e-9** sur les flottants (la CI tourne sur Linux, macOS et
  Windows ; `ln` / `exp` peuvent différer au dernier bit). C'est la raison
  pour laquelle `insta`, déjà utilisé par `sobria-ingest`, n'est pas repris
  ici : il compare les flottants à l'identique.
- Seule la valeur `SOBRIA_UPDATE_GOLDEN=1` réécrit les fichiers, et elle est
  refusée quand `CI` est défini.
- Un test qui échoue signale un changement de comportement. Ce n'est pas
  forcément un bug : un changement voulu (nouveau coefficient, nouveau
  modèle) se valide en régénérant les fichiers **et en le disant dans la PR**.

Régénérer (uniquement si le changement est voulu) :

```bash
SOBRIA_UPDATE_GOLDEN=1 cargo test -p sobria-estimator --test characterization
git diff crates/sobria-estimator/tests/golden   # relire chaque écart
```

```powershell
$env:SOBRIA_UPDATE_GOLDEN = "1"; cargo test -p sobria-estimator --test characterization
Remove-Item Env:SOBRIA_UPDATE_GOLDEN
```

Un test qui réécrit un fichier **échoue exprès** (une variable restée
exportée ne doit pas absorber une régression en silence) : relancer ensuite
sans la variable pour vérifier. Si un test est renommé, supprimer à la main
l'ancien fichier `.json`.

Ces tests ne disent pas si les valeurs sont **justes** (voir le module
`validation` de la crate), seulement qu'elles ne bougent pas.

## English

These JSON files pin the **current behaviour** of `sobria-estimator`
(SOB-9): central value, P5–P95 interval, histograms, hypotheses, errors.
They are read by `tests/characterization/`.

- Fixed inputs: seed `DEFAULT_SEED` (42), frozen request timestamp. The
  `computed_at` field (time of the computation) is masked (`<redacted>`).
- Comparison: exact on integers and strings, **1e-9 relative tolerance** on
  floats (CI runs on Linux, macOS and Windows; `ln` / `exp` can differ in the
  last bit). This is why `insta`, already used by `sobria-ingest`, is not
  reused here: it compares floats exactly.
- Only `SOBRIA_UPDATE_GOLDEN=1` rewrites the files, and it is refused when
  `CI` is set.
- A failing test means the behaviour changed. That is not necessarily a bug:
  an intended change (new coefficient, new model) is accepted by regenerating
  the files **and saying so in the PR**.

Regenerate (only if the change is intended):

```bash
SOBRIA_UPDATE_GOLDEN=1 cargo test -p sobria-estimator --test characterization
git diff crates/sobria-estimator/tests/golden   # review every difference
```

```powershell
$env:SOBRIA_UPDATE_GOLDEN = "1"; cargo test -p sobria-estimator --test characterization
Remove-Item Env:SOBRIA_UPDATE_GOLDEN
```

A test that rewrites a file **fails on purpose** (a variable left exported
must not silently absorb a regression): re-run without the variable to
verify. If a test is renamed, delete the old `.json` file by hand.

These tests do not tell whether the values are **right** (see the crate's
`validation` module), only that they do not move.
