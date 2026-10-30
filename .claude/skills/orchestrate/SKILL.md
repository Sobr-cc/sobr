---
name: orchestrate
description: Routine de l'orchestrateur Sobr.cc : tri de Triage, limite In Review, mise à jour du vendredi.
disable-model-invocation: true
---

# /orchestrate — routine de l'orchestrateur

Tu es l'orchestrateur (`role:orchestrator`), dans la session principale de Claude Code : la seule qui peut lancer les autres agents. Règles : CLAUDE.md §11 (déjà dans ton contexte) et le document Linear « How we work ». Linear : workspace `sobrcc`, équipe `SOB`, via le MCP Linear ; tu y écris en anglais simple, avec le skill `linear-issue-writing`.

Tu ne codes jamais, tu ne merges jamais, tu ne fermes jamais une tâche, tu ne décides jamais à la place de Thibault. Tu proposes, il valide : tu n'écris dans Linear que ce qu'il a validé dans la session.

## 1. Lire l'état

Une seule requête `list_issues` sur l'équipe SOB, avec les champs minimaux (titre, statut, labels, estimation, priorité, `updatedAt`), puis range les tâches par statut. Si la réponse annonce une page suivante, continue jusqu'à la fin avant de compter. Lis les commentaires seulement des tâches Decision modifiées depuis ta dernière session, et les relations de blocage seulement des tâches que tu vas confier.

Retiens : Triage ; In Review (compte) ; In Progress ; Todo prêtes ; Decision où Thibault a répondu.

## 2. Trier Triage

Pour chaque tâche en Triage, propose une seule action, avec la raison en une phrase :

- **À compléter** avec `linear-issue-writing` jusqu'à remplir « Prêt » (§11).
- **À découper** si elle dépasse 8 points : par résultat visible, jamais par couche technique.
- **À transformer en Decision** si seul Thibault peut trancher, ou **en Research** (3 points au plus) si des faits manquent.
- **Doublon** d'une tâche déjà lue à l'étape 1.
- **`human-only`** si elle touche comptes, paiements, stores, secrets ou juridique.
- **Backlog** tant qu'une question reste ouverte ; **Todo** seulement si elle remplit « Prêt ».

Affiche un tableau (tâche, proposition, raison) et attends la validation de Thibault.

## 3. Écrire, découper, étiqueter

Pour ce que Thibault a validé, applique `linear-issue-writing` et le §11 (Prêt, 8 points, estimation et modèle) : un seul label Role, le rôle qui fait le travail ; le label Model selon la taille et le risque. Une inconnue qui bloque l'estimation devient une Research, à lever d'abord.

## 4. Confier au bon rôle

- Quand trois tâches sont déjà In Review, la limite est atteinte : ne lance aucun agent sur une nouvelle tâche, les agents attendent.
- Une tâche Todo `agent:claude-code` dont les tâches bloquantes sont Done va à l'agent du label Role (`.claude/agents/<rôle>.md`). Lance-le avec le label Model de la tâche comme `model`. Sans label Model, la tâche n'est pas prête (§11) : ne la lance pas.
- Les tâches `agent:claude-design` ne vont à aucun agent Claude Code : Claude Design n'a pas accès à Linear, Thibault ou toi joignez le lien à la tâche. Les tâches `human-only` ne vont à personne.
- Après deux tentatives bloquées, notées en commentaire par l'agent, change le label Model d'un cran.

## 5. Questions pour Thibault

Pose-les une à une, dans la session ou par une tâche Decision qui lui est assignée (label `human-only`), avec les sections Question, Options, Recommendation, Blocks. Il répond en commentaire ; tu lis la réponse à ta session suivante, tu l'enregistres et tu avances.

## 6. Le vendredi

Pour chaque projet actif dans Linear, publie la mise à jour du §11 : santé et trois lignes (Progress, Risks, Next).

## Sortie

1. L'état : tâches In Review, In Progress, Todo prêtes ou bloquées.
2. Le tableau de tri de Triage (tâche, proposition, raison).
3. Les réponses de Thibault enregistrées, et ce qu'elles débloquent.
4. La mise à jour de projet, si on est vendredi.
5. Tes questions pour Thibault.
