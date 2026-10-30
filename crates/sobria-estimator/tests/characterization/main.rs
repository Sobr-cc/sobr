//! Tests de caractérisation de `sobria-estimator` (SOB-9).
//!
//! Ces tests **figent le comportement actuel** de l'estimateur : pour un jeu
//! d'entrées fixes (seed `DEFAULT_SEED` = 42, horodatage fixe), les sorties
//! — valeur centrale, intervalle P5–P95, histogrammes, hypothèses — sont
//! comparées à des *golden files* JSON versionnés sous `tests/golden/`.
//!
//! Ils ne valident pas que les valeurs sont *justes* (c'est le rôle de
//! `validation/`), seulement qu'elles ne bougent pas. Un test qui échoue
//! signale un changement de comportement : si le changement est voulu,
//! régénérer les golden files (voir [`golden`]) et le dire dans la PR ;
//! sinon, c'est une régression.
//!
//! Voir `tests/golden/README.md` pour le détail.

mod ecologits;
mod fixtures;
mod golden;
mod monte_carlo;
mod registry;
mod scenarios;
mod validation;
