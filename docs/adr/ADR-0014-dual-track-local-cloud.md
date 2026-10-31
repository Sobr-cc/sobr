# ADR-0014 — Stratégie dual-track : local-first + cloud opt-in

- **Statut** : Accepted
- **Date** : 2026-05-16
- **Décideurs** : Thibault, Cowork
- **Contexte** : après v0.7.0 (Mode Équipe self-hosted shippé), avant la candidature data.gouv.fr v1.0
- **Liens** : CLAUDE.md §7 (privacy by design), §13 (anti-patterns), ADR-0013 (extension + pairing + mode équipe)
- **Amendment 1** (2026-10-30, Accepted): the managed cloud has a 30-day trial, then is paid per seat (condition 5); a personal space on the Sobr server is allowed (condition 1). See the section below.
- **Amendment 3** (2026-10-31, Accepted): the phasing, the feature table and the diagram label follow the calendar of October 30. See the section below.

---

## Amendment 1 — 30-day trial and personal space (2026-10-30)

**Status.** Accepted (2026-10-30); merging the PR is Thibault's approval. The original text is unchanged; notes mark conditions 1 and 5.

**What changes.**

1. **Condition 5 is replaced.** The managed cloud has a 30-day trial, then is paid per seat, with a dedicated price for public bodies and research. The trial needs a payment card to start (SOB-105). Local and self-hosted stay free. A public price and no mandatory annual commitment stay as before; pay-per-use, allowed before, is not part of the new offer, because the arbitration names only payment per seat. The previous version required a real free tier on the managed offer (example: 1,000 estimates per month and 5 users), with no payment card.
2. **Condition 1 is clarified.** A person can have a personal space on the Sobr server, which runs the same binary: it syncs between their machines, and the same account lets them join an organization. Guest mode, local, with no account and no signal to Sobr, stays the default and needs nothing from the server. The previous version described only that guest mode.
3. **The license stays MIT** for all the code (ADR-0006). The paid value comes from the service: managed cloud, support, commitments.

**What does not change.** Conditions 2, 3 and 4 apply as written, including to the personal space: same binary, no prompt text (only metadata and results), hosting in the EU with the GDPR duties.

**Why.** Thibault's arbitrations of October 30 (the offer and the personal account) are in the "Gaps" section of [App and screen architecture](https://claude.ai/artifact/5UaiKHXrKPVyRPEd6gfeyr). The document listed the conflict: this ADR requires a real free offer, so either keep one or amend it explicitly. Thibault chose to amend, with the three changes above. Screens: Z1b, A1, S3, in [Routes, screen by screen](https://claude.ai/artifact/Cua7SK84VzMCwYFZRQfUHD).

**How the Révision rule of this ADR is met.**

1. A written decision by Thibault: the arbitration above, then his approval of the PR.
2. A public justification: this section and the CHANGELOG entry. The README is unchanged for now (see below).
3. 30 days' notice for managed users: nobody uses the managed cloud yet, so nobody is notified. The notice applies to any later change of these conditions.

The Révision section asks for a separate "ADR-0014-amendement" document. This amendment is a section of this file, as SOB-35 asks. If SOB-87 decides to supersede instead, the section moves into a new ADR unchanged.

**Not covered here.** These passages still describe the previous plan or an open point. They need their own decision, and are not edited here. Amendment 3 (SOB-103) settles the README, the diagram label, the pairing row and the phasing; SOB-154 settles the price and the conflict with ADR-0013.

- The README says nothing about the managed offer, and "no Sobr cloud" stays true for the local and self-hosted modes. It changes when the offer is announced (SOB-103).
- The price of the personal space: it runs on the Sobr server, which change 1 makes trial-then-paid, but the arbitration does not say whether it is free or under the trial. Guest mode stays free either way. The screens Z1b, A1 and S3 depend on it (SOB-136).
- The conflict with ADR-0013: its section "Ce qu'on ne fera PAS" says "Pas de comptes utilisateurs hébergés par nous" and `Pas de mode "synchro multi-postes du même utilisateur" en v1`, which the personal space of change 2 contradicts; the "Validation" section of this ADR still says "Cohérence ADR-0013 ✓" (SOB-136).
- The "Modèle technique" diagram says "MIT/Apache"; the license is MIT (change 3; SOB-103).
- The row "Pairing perso 6 chiffres" of the feature table gets its note from the ADR-0013 amendment (SOB-34).
- "Implémentation phasée" (phase 5 lists a free tier, cloud beta and GA at v1.3.0 and v2.0.0, and "no public cloud announcement before v1.3.0") and "Lien avec les chantiers" (the cloud "not mentioned" at the application; the application is now off, SOB-104, so that rule no longer applies) predate the calendar of October 30, which builds the cloud before the public V1 of March 2027. The row "Multi-device" (v1.2.0) is later than the personal space sync between machines. SOB-103 owns these passages.

## Amendment 3 — Phasing and feature table aligned with October 30 (2026-10-31)

**Status.** Accepted (2026-10-31), SOB-103; merging the PR is Thibault's approval. Same form as the previous amendments. It is numbered after Amendment 2 (SOB-154, PR #22).

**What changes.** Each passage that Amendment 1 listed under "Not covered here" is aligned, or kept with its reason.

1. **Phasing.** The calendar of October 30 ([Routes, screen by screen](https://claude.ai/artifact/Cua7SK84VzMCwYFZRQfUHD), "Calendrier") replaces the version numbers of phases 2 to 6 (v1.0.0 to v2.0.0). Organization features and the cloud are built in January and February 2027, the beta and the launch are in March, and the public V1 is on March 31, 2027. The content of phases 0, 1 and 3 is unchanged.
   - Phase 2 (application): dropped. The data.gouv.fr application is off (SOB-104).
   - Phase 4: SSO, SCIM and custom roles are built in the Organization and identity project, and the sync of the personal space between machines in the Cloud and self-hosting project. Both stay in the common open-source binary.
   - Phases 5 and 6: the beta is in March 2027 and the launch is with the public V1. The "free tier" is replaced by the 30-day trial for organizations (Amendment 1) and the free personal space (SOB-154).
2. **Announcement.** "Pas avant v1.3.0, jamais simultanément avec la candidature" becomes: the cloud is announced with the public V1 launch. The second half of the rule goes, since the application is off (SOB-104).
3. **"Lien avec les chantiers".** C-candidature, "the cloud is not mentioned": dropped for the same reason. C-admin-avance and C-cloud-beta are the projects named in point 1; the rule that their code is the common binary stays.
4. **Feature table.** The row "Pairing perso 6 chiffres" is already aligned by ADR-0013 Amendment 1 (SOB-34): kept. The rows "SSO" and "Multi-device" are no longer "v0.8+": they follow the projects of point 1.
5. **License label and "payant".** The license is MIT (ADR-0006), not "MIT/Apache". "Payant" for the managed offer means paid for organizations (trial, then per seat); the personal space is free.
6. **Trial and card.** Kept as Amendment 1, change 1 states it: a payment card starts the organizations' trial (SOB-105). The payment provider account is a human-only task (SOB-131).
7. **README.** Kept as it is: it says nothing about the managed offer, and "no Sobr cloud" stays true for the local and self-hosted modes. It changes at the announcement, with the V1 launch, because an earlier mention would describe an offer that does not exist yet.

**What does not change.** The five conditions, and the rule that the managed cloud runs the same open-source binary. The amendment changes the plan, not a condition, so the Révision rule needs no notice period.

**Not covered here.** The dates of individual builds (they belong to the issues of the projects), the price amounts, and the chantier names C29 and C30, which are history. The mentions of the data.gouv.fr pitch in "Contexte" and "Conséquences" give the reasons of May 2026 and are kept; the application is off (SOB-104).

**Issues.** SOB-103, SOB-104 and SOB-105 (the answers), SOB-87 (amendment by PR), SOB-35, SOB-34, SOB-131.

---

## Contexte et énoncé du problème

ADR-0013 a explicitement rejeté l'**Alt 2 — SaaS Sobr.ia centralisé** au motif d'un conflit avec :

- CLAUDE.md §7 *« privacy by design, tout traitement par défaut local, pas d'envoi externe »* ;
- le pitch défi data.gouv.fr (souveraineté, frugalité) ;
- les coûts infra continus, GDPR, sécurité.

Mais le retour terrain post-v0.7.0 révèle un besoin légitime non couvert :

1. **TPE/PME sans IT** veulent utiliser le Mode Équipe sans déployer ni opérer un serveur ;
2. **Freelances et indépendants** veulent une option managed avec mail SMTP géré, backup auto, monitoring ;
3. **Admins avancés** ont besoin de SSO, RBAC, gestion centralisée des seuils que le mode self-hosted basique ne couvre pas en v0.7.

La question n'est plus *« faut-il proposer du cloud ? »* mais *« comment proposer du cloud sans trahir le pitch souverain et le CLAUDE.md ? »*

---

## Décision

Adopter une **stratégie dual-track « local-first + cloud opt-in »**, inspirée des projets open-source matures (Bitwarden/Vaultwarden, Plausible, PostHog, Cal.com, Linkwarden).

### Principe

- **Le local reste le défaut absolu**. Particulier qui télécharge l'app : aucune inscription, aucun cloud, ça marche en 100 % local. Comportement actuel inchangé.
- **Le cloud Sobr.ia est une offre managed parmi d'autres**. C'est juste l'hébergement par nous du même binaire `sobria-team-aggregator` que tout le monde peut self-hoster gratuitement.
- **Le binaire est identique**. Pas d'« édition cloud premium » avec des features fermées. Pas de fork.
- **L'utilisateur choisit son hébergeur** : lui-même (self-hosted gratuit), Sobr.ia managed (payant), ou un autre prestataire managed (tiers).

> Amendment 3 (2026-10-31): "payant" means paid for organizations; the personal space is free. See point 5.

### 5 conditions impératives

Toute offre cloud Sobr.ia doit respecter ces 5 conditions, faute de quoi le pitch défi data.gouv.fr et CLAUDE.md §7 sont trahis :

1. **Local par défaut** — l'app + l'extension fonctionnent en 100 % local sans inscription, sans compte, sans signal vers Sobr.ia. Le cloud est un choix conscient et explicite.
   > *Amendment 1 (2026-10-30): clarified. A personal space on the Sobr server is allowed; guest mode stays the default. See the Amendment 1 section.*
2. **Binaire open-source identique** — le serveur managé chez Sobr.ia est le même `sobria-team-aggregator` que tout self-hoster. Toute amélioration backend est merge sur la même branche, accessible à tous. Pas de privatisation de features.
3. **Pas d'envoi de prompts en clair** — le cloud Sobr.ia n'ingère que des metadata + résultats (tokens, gCO₂eq, méthodologie), jamais le contenu textuel des prompts. Strict respect du contrat d'extension/app actuel.
4. **GDPR-compliant** — hébergement UE (OVH Gravelines, Scaleway, Clever Cloud), DPO désigné, clauses contractuelles types, privacy policy publique, DPIA documentée, droit à l'effacement implémenté, export RGPD self-service.
5. **Free tier transparent + tarif lisible** — l'offre managed propose un free tier réel (ex : 1 000 estimations/mois, 5 utilisateurs) sans carte bancaire. Au-delà, tarif public en €/utilisateur/mois ou en pay-per-use, sans engagement annuel obligatoire.
   > *Amendment 1 (2026-10-30): replaced. The managed cloud has a 30-day trial, then is paid per seat. The text above is the previous version. See the Amendment 1 section.*

Si une décision produit future viole une de ces 5 conditions, le ADR doit être amendé explicitement par décision de Thibault — pas de dérive silencieuse.

### Modèle technique

```
                            ┌─────────────────────────────────┐
                            │   Binaire sobria-team-aggregator│
                            │   (Rust, open source, MIT/Apache)│
                            └─────────────────────────────────┘
                                          │
                ┌─────────────────────────┼─────────────────────────┐
                ▼                         ▼                         ▼
        ┌──────────────┐          ┌──────────────┐          ┌──────────────┐
        │ Self-hosted  │          │   Managed    │          │  Tiers       │
        │  (gratuit)   │          │   Sobr.ia    │          │  managed     │
        │              │          │  (payant)    │          │  (libre)     │
        │ TPE/PME IT,  │          │ TPE/PME sans │          │ Hébergeurs   │
        │ DSI, souver. │          │ IT, indép.,  │          │ tiers, ESN   │
        │              │          │ freelances   │          │              │
        └──────────────┘          └──────────────┘          └──────────────┘
                │                         │                         │
                └─────────────────────────┴─────────────────────────┘
                                          │
                            ┌─────────────▼─────────────┐
                            │  Extension navigateur     │
                            │  + app Tauri personnelle  │
                            │  (parlent à n'importe     │
                            │  quel des 3 par URL)      │
                            └───────────────────────────┘
```

> Amendment 3 (2026-10-31): the license is MIT, not MIT/Apache. See point 5.

Côté client (extension + app), aucune notion de « Sobr.ia cloud » n'est privilégiée. L'URL serveur est saisie librement par l'utilisateur. La seule différence pour l'offre managed Sobr.ia : `https://team.sobr.ia` est dans la liste des serveurs suggérés en autocomplete, comme on pourrait suggérer `https://sobria.ovh.fr/xxx`.

### Différenciation features

Le **backend** est identique partout. La différenciation managed se fait sur l'**infrastructure** opérée par Sobr.ia :

| Capacité | Self-hosted | Managed Sobr.ia |
|---|---|---|
| Binaire open-source | ✓ | ✓ |
| Pairing perso 6 chiffres (automatic, no code, since ADR-0013 Amendment 1, 2026-10-30) | ✓ | ✓ |
| Mode Équipe (enrollment codes, JWT, dashboard admin) | ✓ | ✓ |
| Exports CSRD/PROV-O/CSV | ✓ | ✓ |
| Alertes seuils (webhook, log) | ✓ | ✓ |
| Alertes mail SMTP | configuration manuelle | **SMTP géré inclus** |
| Backup automatique SQLite | à la charge de l'admin | **Backup chiffré quotidien S3 inclus** |
| Monitoring uptime + alertes ops | à la charge de l'admin | **Statuspage publique + PagerDuty inclus** |
| Update du référentiel modèles (Gold) | `dvc pull` manuel | **Update automatique sans action** |
| Support technique | communauté GitHub | **Support email/chat 24h** |
| SSO (SAML, OIDC) | code commun v0.8+ | **Pré-configuré avec providers populaires** |
| Multi-device | code commun v0.8+ | **Pré-configuré** |
| Sauvegarde géo-redondante | à la charge de l'admin | **3 zones UE inclus** |

> Amendment 3 (2026-10-31): the rows "SSO" and "Multi-device" follow their projects. See point 4.

L'offre managed apporte de l'**opex géré** et du **support**, pas du code propriétaire. Tout le code reste open-source et installable gratuitement.

---

## Alternatives rejetées (rappel)

### Alt 1 — Rester 100 % local-only, refuser le cloud

Cohérent avec ADR-0013 originel mais ne couvre pas le besoin TPE/PME sans IT. À terme, on perd ces utilisateurs au profit de SaaS concurrents qui n'ont aucun scrupule sur la privacy. Rejeté.

### Alt 2 — SaaS Sobr.ia centralisé propriétaire

Cloud as default, fork du backend en édition premium. Conflit frontal CLAUDE.md §7 + ADR-0013. Rejeté définitivement.

### Alt 3 — Cloud opt-in avec code open-source identique (CHOISI)

Le compromis qui préserve les principes et étend le marché. Choisi.

---

## Conséquences

**Positives** :
- Élargit la cible utilisateurs (particuliers ET TPE/PME sans IT ET DSI ET freelances).
- Préserve intégralement le pitch souverain : le local-first reste le défaut, le cloud est un choix conscient.
- Le binaire reste 100 % open-source, auditable, fork-able. La communauté self-hosting reste vivante.
- Création potentielle d'une source de revenus pérenne pour financer le développement open-source (modèle Plausible/Cal.com).
- Renforce le pitch défi data.gouv.fr : *« 100 % open-source, self-hostable ou managed, vos données souveraines dans tous les cas. »*

**Négatives** :
- Coûts opex managed (serveurs, support, monitoring, backup) à amortir.
- GDPR + DPO + DPA à constituer pour le cloud Sobr.ia.
- Sécurité opérationnelle (pentest annuel, bug bounty éventuel).
- Risque réputationnel si une faille touche le cloud Sobr.ia, même si le binaire est partagé.
- Maintenance double UX : il faut tester self-hosted ET managed à chaque release.

**Neutres** :
- Aucun impact sur l'app perso et l'extension actuelles. Le cloud est invisible tant qu'on ne l'active pas.

---

## Implémentation phasée

| Phase | Version | Chantier | Périmètre |
|---|---|---|---|
| **0 — Acter** | — | ADR-0014 (ce doc) | Décision figée par écrit |
| 1 — Polish | v0.7.1 | C29 (en cours) | UI Mode Équipe + admin reset-password + regen-cert + alertes seuils (cf. ADR-0013 Phase 2 enrichie) |
| 2 — Candidature | v1.0.0 | C-candidature | Sprint final data.gouv.fr (dossier + vidéo + UAT). **Pitch souverain pur**, le cloud n'est pas annoncé. |
| 3 — Doc self-hosting accessible | v1.1.0 | C-doc-quickstart | Quickstart « 5 minutes » non-IT pour le self-hosted (complément `docs/operations/team-aggregator.md`). |
| 4 — Admin avancée commune | v1.2.0 | C-admin-avance | SSO (SAML/OIDC), RBAC fin, multi-device — dans le binaire open-source commun. |
| 5 — Cloud beta managed | v1.3.0 | C-cloud-beta | Infrastructure managed (OVH/Scaleway), Stripe, landing page, privacy policy, DPIA, free tier. Beta sur invitation. |
| 6 — Cloud GA | v2.0.0 | C-cloud-ga | Public, marketing, support, statuspage, bug bounty. |

> Amendment 3 (2026-10-31): the version numbers of phases 2 to 6 and the "free tier" are replaced by the calendar, and phase 2 is dropped. See point 1.

**Annonce publique cloud** : pas avant v1.3.0, jamais simultanément avec la candidature data.gouv.fr (dilution du pitch souverain).

> Amendment 3 (2026-10-31): the cloud is announced with the public V1 launch; the part about the application no longer applies. See point 2.

---

## Validation

- Cohérence CLAUDE.md §7 ✓ — local par défaut, cloud opt-in explicite.
- Cohérence ADR-0013 ✓ — l'Alt 2 reste rejetée (cloud propriétaire fermé). L'Alt 3 (cloud open-source) est un raffinement, pas une contradiction.
- Couverture besoin TPE/PME sans IT ✓ — offre managed.
- Couverture besoin souverain ✓ — self-hosted reste l'option par défaut, gratuit, libre.
- Modèle économique soutenable ✓ — opex managé, pas de privatisation de features.

---

## Lien avec les chantiers en cours et à venir

- **C29 (v0.7.1)** : pas impacté. Continue tel que prévu.
- **C30 (état des lieux datasets)** : à exécuter après v1.0, enrichit le référentiel Gold qui sera diffusé via DVC à tous les utilisateurs (self-hosted ET managed). Pas de différenciation.
- **C-candidature (v1.0)** : pitch souverain pur, le cloud n'est pas mentionné.
- **C-admin-avance (v1.2)** : SSO, RBAC, multi-device. **Open-source dans le binaire commun**, pas exclusif managed.
- **C-cloud-beta (v1.3)** : infrastructure managed dédiée. Code = binaire commun.

> Amendment 3 (2026-10-31): see point 3.

---

## Révision

Ce ADR est révisable mais pas amendable silencieusement. Toute modification d'une des 5 conditions impératives nécessite :

1. Une décision écrite de Thibault dans un nouveau ADR-0014-amendement.
2. Une justification publique dans le CHANGELOG + README.
3. Une période de préavis de 30 jours pour les utilisateurs managed avant application.

Le but est de **garantir aux utilisateurs que les règles du jeu ne changeront pas en silence** — c'est ce qui distingue Sobr.ia d'un SaaS classique.
