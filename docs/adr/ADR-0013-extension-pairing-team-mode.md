# ADR-0013 — Extension navigateur + pairing local + mode Équipe self-hosted

- **Statut** : Phase 1 Implemented (v0.6.0, 2026-05-16) ; Phase 2 Implemented (v0.7.0, 2026-05-16, C28) ; Phase 3 Backlog (v0.8+)
- **Amendment 1** (Accepted 2026-10-30, SOB-34): automatic pairing replaces the 6-digit code of stage 1. See the section below.
- **Date** : 2026-05-15
- **Décideurs** : Thibault, Cowork
- **Contexte** : avant chantier C27 (v0.6.0 extension) et C28 (v0.7.0 mode équipe)

## Amendment 1 — Automatic pairing (2026-10-30)

**Status.** Accepted (2026-10-30), SOB-34.
- The original text below is kept as it was, and notes mark the parts this
  amendment replaces.
- The amendment is written in place because SOB-34 asks for it, although
  `docs/adr/README.md` says an Accepted ADR is changed by a new one.
  SOB-87 decides between the two forms; this section can move unchanged
  into a new ADR.
- It is in English under the October 30 language rule.

**What changes.** Stage 1, steps 1 to 3: the person no longer types a 6-digit
code. After a single choice in the app, the extension pairs with the app on
its own. Alternative 4 ("native messaging with auto-pairing, no code"),
rejected in 2026-05, becomes the rule.

**When and why.** On October 30, Thibault ruled on the row "Appairage (E2)" in
the "Écarts" section of the
[App and screen architecture](https://claude.ai/artifact/5UaiKHXrKPVyRPEd6gfeyr)
document. Translated from French:

> Fully automatic; the bridge manifest only allows the Sobr extension;
> unpairing in the settings.

- **The option he did not take:** the same document had recommended
  "automatic detection, then confirmation by the code". He chose to accept
  the risk of having no code, under the conditions below.
- **Screen E2** in [Routes, screen by screen](https://claude.ai/artifact/Cua7SK84VzMCwYFZRQfUHD)
  says the same thing: the app is detected and the connection is automatic;
  otherwise the extension suggests the app and works without tracking.

**What the previous version required.**

- **This ADR required:**
  - the app showed a 6-digit code (about 20 bits, valid 5 minutes, single
    use) and the person typed it in the extension;
  - the code was the anti-spoofing step: alternative 4 was rejected because
    nothing stopped a malicious extension on the same machine from posing
    as Sobr;
  - the manifests were written with the person's explicit consent.
- **The code also designed, but no screen ever shipped:** the person would
  type a Chrome extension ID. The app would write that single ID into the
  manifests of all Chromium-family browsers.

### Decision

1. **One explicit choice, then automatic pairing.**
   - **Consent.** The opt-in required by CLAUDE.md §7 and ADR-0015 ("opt-in
     at each stage") is one explicit choice in the app: "Connect my
     browsers", offered at first launch (A1) and in Settings. This choice
     writes the native messaging manifests.
   - **Pairing.** From then on, when the extension reaches the bridge
     (`connectNative`) and the app answers, the extension sends a pairing
     request with no code. The request carries a random identifier for its
     installation, which means one browser profile. The app creates the
     pairing and returns the secret, as today: 256 bits, stored as an
     Argon2id hash on the app side.
   - **Without the app,** the extension shows E2's suggestion and keeps
     working without tracking. Nothing is sent anywhere.
2. **The bridge manifest only allows the Sobr extension.**
   - The app writes the manifests itself, with the published Sobr extension
     IDs; the person types nothing.
   - Every Chromium-family manifest lists both the Chrome Web Store ID and
     the Edge Add-ons ID, because Edge can also install from the Chrome Web
     Store.
   - The Firefox manifest lists the Firefox ID in `allowed_extensions`.
   - The names (native host, Firefox ID) follow ADR-0020 if it is accepted,
     and stay as today otherwise.
3. **Unpairing is done in the settings, on either side, and it sticks for
   that installation.**
   - **In the app,** the Settings page lists paired browser profiles, each
     with an "Unpair" action. It revokes the secret and marks that
     installation's identifier as revoked, so the same installation is not
     paired again automatically. Only "Reconnect" in the same list lets it
     pair again.
   - **In the extension,** "Disconnect" switches automatic pairing off for
     that installation and sends the revoke. If the app is closed, the revoke
     waits and is sent when the app is next reached.
   - **Either way,** once the app has the revoke, it accepts no more
     estimates from that installation.
   - **After a reinstall or a reset of the extension's storage,** the
     extension has a new identifier. While that browser is still connected
     in the app's Settings, it pairs again, with the notice below and a new
     line in the app's Settings, so it is never silent.
   - **To stop a whole browser,** the person removes it in the app's
     Settings. The app revokes every installation recorded for that browser
     and refuses new pairing requests from it until the person adds it back.
     It also deletes that browser's manifest. Adding the browser back lifts
     only the refusal: its installations pair again automatically, each with
     a new secret, and the revoked secrets stay revoked. Installations
     unpaired one by one stay unpaired until "Reconnect".
   - **Turning "Connect my browsers" off** deletes all the manifests, and
     the app refuses every pairing request and estimate until it is turned
     on again. A bridge that a browser already started keeps running until
     the browser closes the port, so deleting the manifests alone would not
     stop it. What such a bridge writes to the offline spool is refused too:
     while the connection is off, the app discards spooled lines instead of
     keeping them, and when it is turned back on, it drops the lines written
     before that moment.
   - **The refusal in the app is what stops the browser, not the deleted
     manifest.** On Windows, Edge reads the Chromium and then the Chrome
     registry keys when its own key has no manifest
     ([Microsoft](https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/native-messaging)).
     Those manifests allow Edge's ID (Decision 2), so while Chrome or Chromium
     is connected, Edge still reaches the bridge after its own manifest is
     deleted. Leaving Edge's ID out of them would not help: Edge can run the
     Chrome Web Store build, which has Chrome's ID.
   - **How the app knows the browser.** Each pairing request also names the
     browser it comes from, since the extension ID does not tell Edge from
     Chrome. The extension reads it from the browser: Edge lists "Microsoft
     Edge" in `navigator.userAgentData.brands`
     ([Microsoft](https://learn.microsoft.com/en-us/microsoft-edge/web-platform/user-agent-guidance)),
     and Firefox has `runtime.getBrowserInfo()`. The implementation must
     identify each supported browser this way.
   - **The app accepts a pairing request only if it names a supported
     browser that is connected in Settings.** A missing or unknown browser
     is refused. The browser is self-reported, which is enough under the
     threat model in "Why the code is no longer needed": only Sobr's own
     extension reaches the bridge, and processes of the same OS user are out
     of scope.

### Why the code is no longer needed

- **The allow-list already existed; what changes is where the ID comes
  from.** The manifest allow-list (`allowed_origins`,
  `allowed_extensions`) was already there, and the browser refuses
  `connectNative` to any extension it does not list. The ID was meant to be
  typed by the person, so it could have been a wrong or malicious one. From
  now on it is fixed and written by the app.
- **What an attacker would still need.** An unpacked Chromium extension
  that copies Sobr's public key, or a temporary Firefox add-on, can claim
  the same ID. Both need the person's own action in developer mode, or a
  program already running as that person.
- **The threat model.** Processes of the same OS user are out of scope:
  they can already read the app's data and the extension's storage, with or
  without a code.
- **No other way in.** An extension cannot reach the app's pipe, socket or
  spool: it has no access to native code or to the file system.
- **The code was weak anyway.** There was no limit on failed attempts, and
  a refused attempt kept the code valid (SOB-7 audit, SOB-91).
- **An optional extra check.** The bridge can pass to the app the caller ID
  that the browser gives it at start-up.

### Conditions

Automatic pairing ships only with these conditions met:

- **The IDs exist.** The Chrome Web Store and Edge Add-ons listings are
  created and their IDs recorded, and the Firefox ID is reserved. All three
  are human-only steps, done before the app ships manifests that name them.
- **The bridge ships with the app,** and the app installs it when the person
  makes the choice in Decision 1. Today it is not bundled and no screen
  installs it (SOB-7 audit).
- **One manifest per browser.** Today Windows shares one file across
  browsers. The app removes the manifests when the person turns the
  connection off, and so does an uninstaller where one exists. A manifest
  left after an uninstall is harmless: the bridge it points to is gone.
- **Stopping a browser holds in the app.** The app records the browser of
  each installation, accepts pairing requests only from a supported browser
  that is connected, and revokes every installation recorded for a browser
  the person removes (Decision 3). Deleting its manifest is not enough: on
  Windows, Edge falls back to the Chromium and Chrome keys, and a bridge
  already started keeps running.
- **The app–bridge channels are reserved to the current OS user:**
  - on Windows, a pipe whose name includes the user and whose security
    descriptor is limited to that user; the bridge checks who owns it before
    sending anything;
  - on macOS and Linux, a socket in a private directory (`0700`, under
    `$XDG_RUNTIME_DIR` or the app data folder), never `/tmp`;
  - the offline spool directory is private in the same way.
- **One pairing per browser profile.** Two browsers that both pair
  automatically would otherwise keep replacing each other. Today every
  pairing has the same fingerprint (SOB-7 audit).
- **The person is told.** The extension shows "Connected to the Sobr app"
  the first time, and the browser appears in the app's Settings. No estimate
  is sent before the pairing exists.
- **The secret is always checked.** Today the offline spool accepts any line
  that carries an estimate while a pairing is active, without checking its
  secret (SOB-7 audit). This bug exists with or without a code, but
  automatic pairing makes it easier to reach.
- **Nothing measured while the connection is off is kept.** The app
  discards the offline spool while "Connect my browsers" is off, and drops
  the lines written before it is turned back on (Decision 3). Today the
  drain does not look at when a line was written (`drain_spool` in
  `extension_store.rs`).
- **The secret never leaves storage.** Today the extension's "Export JSON"
  includes the secret (SOB-7 audit). It must leave it out.

### Consequences

- **Positive:**
  - no code and no extension ID to type, so E1 leads straight to E2;
  - one pairing per browser profile;
  - unpairing that sticks for each installation, and a way to stop a whole
    browser;
  - expected: fewer support cases (expired code, wrong ID).
- **Negative:**
  - protection against spoofing now rests on the store IDs, the manifest
    allow-list and separation between OS users, so the conditions above are
    security requirements;
  - an extension someone builds and loads themselves gets another ID and
    cannot pair;
  - development builds need a pinned Chromium ID. The Firefox development
    and release builds already share one ID.
  - the app writes manifests only for the browsers it supports: today
    Chrome, Edge, Brave, Chromium and Firefox. Opera and Vivaldi install from
    the Chrome Web Store with the same ID, but the app writes no manifest
    for them and refuses a pairing request that names them, so they are not
    supported until the app adds them, and E2 must not suggest the app
    there.
- **Migration:** no outside user has a pairing, since nothing has been
  published (ADR-0020). Development pairings, all stored under the old fixed
  fingerprint, are revoked at upgrade, and each browser then pairs again
  automatically. The code screens are removed from the app and the
  extension, and the code tests (SOB-80, SOB-91, SOB-92) are replaced when
  the change is built.
- **Scope:** the code changes belong to the Browser extension project, not
  to this ADR.
- **ADR-0014:** this amendment adds a note to the row "Pairing perso 6
  chiffres" of ADR-0014's capability table (an orchestrator decision; PR #10
  for SOB-35 leaves that row to this PR).

## Contexte et énoncé du problème

Sobr.ia v0.5.0 ship une app Tauri et un pipeline médaillon de référentiel. Le CDC §M6 prévoit une extension navigateur pour mesurer l'empreinte des prompts en direct sur ChatGPT, Claude, Le Chat. Deux cas d'usage à servir :

1. **Particulier** : l'utilisateur installe l'app Tauri + l'extension sur ses navigateurs. Il veut que les estimations de l'extension remontent dans le Dashboard/Journal de son app personnelle sans configuration complexe.
2. **Entreprise** : une équipe (5–500 personnes) veut centraliser les usages IA de ses collaborateurs pour reporting CSRD, audit FinOps, gouvernance. Chaque employé garde la visibilité de son propre usage. L'admin agrège.

Le CLAUDE.md §7 (privacy by design) interdit :
- l'envoi de prompts utilisateurs vers un serveur externe ;
- tout tracking sans opt-in explicite ;
- la création d'un cloud Sobr.ia centralisé.

Le pitch défi data.gouv.fr repose sur la souveraineté et la frugalité — un SaaS centralisé serait incohérent.

## Décision

**Architecture hybride à deux étages**, opt-in par étage, sans cloud Sobr.ia.

### Étage 1 — Particulier (v0.6.0, C27)

> Steps 1 to 3 below (the 6-digit code) are replaced by Amendment 1
> (2026-10-30): pairing is automatic.

L'extension et l'app personnelle se **pairent localement par code à 6 chiffres**, via le mécanisme **native messaging** standard des WebExtensions (sécurité OS, pas de port ouvert).

Flux :

1. L'app Tauri, au premier lancement après v0.6.0, génère un code à 6 chiffres (entropie 20 bits, suffisant pour usage local éphémère) et propose à l'utilisateur d'installer le bridge natif. L'app écrit alors les fichiers manifest natifs aux bons emplacements OS (Chrome, Firefox, Edge si présents) — avec consentement explicite.
2. L'utilisateur installe l'extension Chrome/Firefox.
3. Au premier ouvrir-popup, l'extension détecte le bridge natif via `chrome.runtime.connectNative`. Si OK, elle demande à l'utilisateur de saisir le code 6 chiffres affiché dans son app. Si le code est correct, le pairing est établi et un secret partagé (32 bytes random, stocké dans `chrome.storage.local` côté extension et SQLite côté app) remplace le code temporaire.
4. À chaque estimation extension, le secret est envoyé avec la charge dans la même session native messaging. L'app vérifie le secret avant d'ingérer.
5. Le secret peut être révoqué côté app (bouton "Dépaire l'extension" dans `/parametres`).

Les estimations ingérées apparaissent dans Dashboard M15 et Journal avec un tag `source = 'extension'`. Filtres "Toutes / App / Extension" disponibles.

**Garanties** :
- Aucun trafic réseau (tout est native messaging local OS).
- L'utilisateur peut désinstaller l'extension ou révoquer le pairing à tout moment.
- Pas de compte, pas d'inscription, pas de cloud.

### Étage 2 — Équipe self-hosted (v0.7.0, C28)

Pour le cas entreprise, un nouveau binaire **`sobria-team-aggregator`** (crate Rust, ~10 MB, équivalent serveur HTTPS local). L'entreprise déploie ce binaire sur SON infrastructure (poste admin, serveur LAN, VPS interne). **Aucun service Sobr.ia n'est impliqué.**

Flux :

1. L'admin lance `sobria-team-aggregator init` qui :
   - Génère une paire TLS auto-signée (ou import certif fourni).
   - Crée une base SQLite locale `team.sqlite`.
   - Affiche l'URL d'accès (ex: `https://192.168.1.42:8443`) et un mot de passe admin initial.
2. L'admin accède au dashboard web (Svelte servi par le binaire), crée N "enrollment codes" (jetons à 12 chiffres, valides 7 jours).
3. Chaque employé reçoit son enrollment code (mail, chat interne).
4. Dans son extension OU dans son app Sobr.ia : section "Mode Équipe" → saisit l'URL serveur (`https://192.168.1.42:8443`) + son enrollment code.
5. L'extension/app obtient un token JWT court (24 h, refresh auto) signé par le serveur. Toutes les estimations sont POST en JSON via HTTPS.
6. Le serveur agrège, affiche le dashboard admin (qui a fait combien d'estimations, modèles populaires, tendances CSRD).
7. Chaque employé peut consulter SON usage personnel via le dashboard (auth par token JWT).

**Garanties** :
- Le serveur est CHEZ l'entreprise, déployé par eux, contrôlé par eux.
- Sobr.ia n'a aucun accès aux données.
- Compatible télétravail (VPN ou exposition HTTPS publique gérée par l'entreprise).
- L'employé peut continuer à utiliser son app perso en local (les deux flux coexistent : pairing perso ET enrollment équipe).
- Code source du serveur 100 % open source (même crate Rust, build reproductible).

### Ce qu'on ne fera PAS

- Pas de cloud Sobr.ia centralisé (SaaS).
- Pas de comptes utilisateurs hébergés par nous.
- Pas de télémétrie vers Sobr.ia depuis le serveur entreprise.
- Pas de pairing par OAuth/SSO d'entreprise en v1 — JWT simple suffit. SSO en v0.8+.
- Pas de mode "synchro multi-postes du même utilisateur" en v1 — l'utilisateur s'attache à UN device. Multi-device en v0.8+.

## Alternatives rejetées

### Alt 1 — 100 % local sans mode équipe

Respecte le CLAUDE.md mais ne couvre pas le cas entreprise. Or le défi data.gouv.fr met en avant les cas d'usage CSRD/AGEC qui sont par essence collectifs. Rejeté.

### Alt 2 — SaaS Sobr.ia (cloud centralisé)

Conflit frontal avec le CLAUDE.md §7 et le pitch défi data.gouv.fr. Coûts infra + GDPR + sécurité à porter. Rejeté.

### Alt 3 — mDNS / découverte LAN uniquement

Trop limitant (pas de télétravail), trop fragile (mDNS bloqué dans beaucoup de réseaux d'entreprise). Rejeté.

### Alt 4 — Native messaging avec auto-pairing (sans code)

> Reversed by Amendment 1 (2026-10-30): this alternative is now the rule. The
> manifest's allow-list answers the spoofing objection below.

Plus simple mais aucune protection contre une extension malveillante installée sur le même OS qui se ferait passer pour Sobr.ia. Le code de pairing est un anti-spoofing utile. Retenu.

## Conséquences

> Amendment 1 (2026-10-30): the points about the 6-digit code (one code to
> type, its short lifetime) no longer apply.

**Positives** :
- Le particulier a une expérience fluide (1 install + 1 code à saisir).
- L'entreprise a un mode souverain sans cloud Sobr.ia.
- Notre pitch défi data.gouv.fr reste cohérent (frugalité, souveraineté).
- Le secret partagé chiffré (Argon2 côté app SQLite) rend le pairing révocable.
- Le serveur entreprise est code open source — auditable, déployable, modifiable.

**Négatives** :
- Le binaire `sobria-team-aggregator` ajoute une crate Rust à maintenir (HTTPS, JWT, dashboard Svelte intégré).
- Le code 6 chiffres est court → on doit le rendre éphémère (TTL 5 min, régénération automatique) pour limiter le brute force, même si c'est local.
- Le mode équipe ajoute un protocole HTTPS — il faut documenter sécurité TLS, rotation certificats, etc.
- L'admin doit auto-héberger : ce n'est pas une expérience "1 clic".

**Neutres** :
- v0.6.0 ship perso seul. v0.7.0 ajoute équipe. Pas d'effet sur l'app actuelle.

## Implémentation phasée

| Phase | Version | Chantier | Périmètre |
|-------|---------|----------|-----------|
| Phase 1 | v0.6.0 | C27 | Extension + pairing perso 6 chiffres + bridge natif + ingestion Journal/Dashboard |
| Phase 2 | v0.7.0 | C28 | `sobria-team-aggregator` (binaire HTTPS standalone) + JWT 24h/refresh 7j + Argon2id partout + dashboard Svelte embedded (4 cards + 3 charts SVG) + exports CSRD PDF + PROV-O JSON-LD + CSV + mode Équipe extension/app (URL + ping + enroll + dispatch local/team/both + logout). Voir `docs/archive/chantiers/C28-mode-equipe-self-hosted.md` et `docs/operations/team-aggregator.md`. |
| Phase 3 | v0.8.0 | C29 (TBD) | SSO entreprise (SAML/OIDC), multi-device, RBAC fin |

## Liens

- CLAUDE.md §7 (privacy by design), §13 (anti-patterns).
- CDC §M6 (Extension navigateur).
- ADR-0012 (multi-méthodologie) — l'extension respecte le catalogue méthodo.
- Brief C27 `docs/archive/chantiers/C27-extension-navigateur.md`.
- Brief C28 (à rédiger pour v0.7.0).

## Validation

> Amendment 1 (2026-10-30): the explicit opt-in of stage 1 is now the one-time
> "Connect my browsers" choice in the app.

- Cohérence CLAUDE.md §7 ✓ (pas de cloud central, opt-in explicite à chaque étage).
- Cohérence pitch data.gouv.fr ✓ (souveraineté, code ouvert, self-hosted).
- Couverture cas particulier ✓ (pairing simple).
- Couverture cas entreprise ✓ (self-hosted aggregator).
- Pas de blocage roadmap v1.0 (extension est un nice-to-have, mode équipe est différentiateur).
