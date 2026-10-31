# ADR-0016 — Politique de visibilité par déploiement (étend ADR-0015)

- **Statut** : Accepted (2026-06-12) — implémenté en C44
- **Décideurs** : Thibault, Cowork
- **Contexte** : décision produit du 2026-06-12 — « voir clairement la
  consommation par employé et par projet ». Étend ADR-0015 sans le
  renverser : le défaut reste protecteur, l'identification intégrale
  devient possible pour les organisations qui en assument le cadre légal.
- **Amendment 1** (2026-10-31, Accepted): the breakdown by project appears only when every group shown has at least k contributors. See the section below.

## Amendment 1 — Breakdown by project: every group must reach k (2026-10-31)

**Status.** Accepted (2026-10-31), SOB-153; merging the PR is Thibault's approval. Written in place by PR (SOB-87): the original text is kept, and a note marks the rule that changes. The header line "implémenté en C44" describes the previous rule; the new one is not implemented yet (code work below).

**What changes.** In the `anonymous` and `opt_in` policies, the rule of the "dimension projet" (C44) is replaced:

1. A space's breakdown by project appears only when every group shown has at least k distinct contributors (k as in ADR-0015, rule 2). The groups are the projects shown, the remainder ("other projects") and "no project". A group's contributors are the distinct people across all the projects it holds, not the largest count among them. Otherwise only the organization total shows. The organization-level check of ADR-0015 comes first: when fewer than k people were active, nothing shows.
2. Empty groups are left out of the check.
3. Under `identified`, the live views are unchanged, since k-anonymity does not apply there. The totals kept after the retention period follow the same all-groups rule whatever the policy (ADR-0022, section 10).

The check is made for each day, as ADR-0022 sets for tasks and projects (section 10): a longer view adds up only the days that passed, so "month minus the days shown" gives nothing back. The same section sets the retention of kept totals.

**Previous wording.** « les agrégats par projet n'affichent que les projets comptant ≥ k contributeurs distincts (modes 1-2), les autres étant fondus dans « autres projets » ».

**Why.** Merging only the small projects leaves a hole: the organization total minus the projects shown gives the merged line, which can hold one or two people. The code shows it: `project_breakdown` (`storage/analytics.rs`) gives the merged line the largest contributor count among the projects it merged, so the line always stands below k. Thibault chose option 1 on SOB-148 (October 31): the same rule as ADR-0022 at task level. The alternative, which keeps the previous rule, shows more breakdowns but lets anyone subtract the projects shown from the total.

**Cost.** Under the default policy, a space whose projects have few people shows its total only. A "no project" group below k hides the breakdown of the whole space too, since it must pass like the others; ADR-0022 accepts this. A member still sees their own usage by project (ADR-0015).

**Code work that follows.** This ADR changes no code. The work is in the issues of ADR-0022 that touch the team server ("Issues to create": 6, 7 and 13 apply the rule), and in SOB-114 (exports) and SOB-115 (analytics, alerts, user list), which apply the visibility policy and k to those outputs. All of it waits for the Deep cleanup of the team server (SOB-25), under the Foundations rule; `project_breakdown` and its test `project_breakdown_folds_below_k` change in those issues, not before. So does the admin dashboard text that tells admins that projects below k fold into "Autres projets" (`web-team/src/routes/admin/dashboard/+page.svelte`, moving to `web/` under ADR-0021).

**Issues.** SOB-153, SOB-148 (the answer), SOB-143 (ADR-0022), SOB-114, SOB-115, SOB-25.

## Contexte et problème

ADR-0015 fixe un comportement unique (k-anonymat + opt-in salarié). Or les
contextes de déploiement diffèrent : une PME de 6 personnes volontaires,
un grand groupe avec DPO et accord CSE, une collectivité. Imposer un seul
réglage soit sur-protège (frustration de l'opérateur légitime), soit
sous-protège (si on inversait le défaut).

## Décision

Une clé de configuration serveur `visibility_policy`, choisie à
l'initialisation (`init --visibility-policy …`) et modifiable ensuite
(`config set`), trois valeurs :

1. **`anonymous`** — strict : agrégats k-anonymes uniquement. Aucune
   identification individuelle côté admin, même volontaire (les opt-in
   sont ignorés à l'affichage).
2. **`opt_in`** — **défaut** : comportement ADR-0015 inchangé. k-anonymat
   sur les agrégats ; n'apparaissent nommément que les salariés ayant
   activé eux-mêmes le partage (révocable).
3. **`identified`** — nominatif intégral : vues par employé sans
   consentement individuel préalable dans l'outil. **Activation refusée
   sans attestation** : l'opérateur fournit une attestation explicite
   (`--attest "…"`) déclarant que le CSE a été informé/consulté
   (L2312-38) et les salariés informés individuellement (L1222-4).
   L'attestation (texte, date, admin) est stockée en base et affichée
   dans le dashboard. Le grain temporel minimal reste le jour, les
   prompts ne sont jamais collectés (invariants ADR-0013/0015).

Règles transverses :

- Le k-anonymat des agrégats s'applique en `anonymous` et `opt_in` ;
  en `identified` il est sans objet (les données individuelles sont
  visibles par politique) et désactivé.
- La **dimension projet** (C44) suit la même politique : les agrégats
  par projet n'affichent que les projets comptant ≥ k contributeurs
  distincts (modes 1-2), les autres étant fondus dans « autres projets ».
  > Amendment 1 (2026-10-31): replaced by the all-groups rule. See point 1 of the Amendment 1 section.
- La politique est affichée en permanence dans le dashboard admin et
  l'espace salarié (transparence : chacun sait sous quel régime il est).
- Le site et la doc présentent toujours `opt_in` comme défaut ; le mode
  `identified` n'est jamais un argument commercial premier.

## Conséquences

- (+) Le besoin opérationnel (« qui consomme quoi, par projet ») est
  servi sans imposer la surveillance par défaut à tous les déployeurs.
- (+) La responsabilité juridique est portée par celui qui la détient
  réellement : l'employeur, via une attestation tracée.
- (−) Trois chemins de code à tester (matrice de politique).
- (−) L'attestation est déclarative : Sobr.ia ne peut pas vérifier la
  réalité de l'information du CSE — assumé et documenté.
  > Amendment 1 (2026-10-31), a further (−): under the default policy, a small space may see its total only.

## Options rejetées

- **Nominatif par défaut** : renverse ADR-0015, contredit la vitrine et
  la candidature, expose les déployeurs négligents.
- **Consentement « coché par l'admin » au nom des salariés** : le
  consentement individuel n'appartient qu'au salarié ; ici c'est une
  POLITIQUE d'organisation assumée comme telle, pas un pseudo-consentement.
