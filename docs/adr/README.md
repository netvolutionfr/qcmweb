# Décisions d'architecture

Une ADR (*Architecture Decision Record*) consigne une décision structurante, son
contexte et ses conséquences. Elle répond à la question que se posera quelqu'un
— toi dans six mois, un collègue, un agent — devant un choix qui ne va pas de
soi : **pourquoi est-ce fait ainsi plutôt que de la manière évidente ?**

## Convention

Un fichier par décision, numéroté : `NNNN-titre-en-kebab-case.md`.

Une ADR est **immuable une fois acceptée**. Une décision qui change ne se
réécrit pas : on en rédige une nouvelle, et l'ancienne passe en *Remplacée par
ADR-NNNN*. L'historique des décisions abandonnées a autant de valeur que celui
des décisions en vigueur — il évite de refaire deux fois le même détour.

Statuts : `Proposée`, `Acceptée`, `Remplacée par ADR-NNNN`, `Abandonnée`.

Ce qui mérite une ADR : un choix qu'on pourrait raisonnablement contester, une
contrainte non évidente, un compromis assumé. Ce qui n'en mérite pas : un choix
sans alternative sérieuse, ou une décision déjà argumentée dans [SPEC.md](../../SPEC.md).

## Index

| # | Décision | Statut |
|---|---|---|
| [0001](0001-pseudonymisation-par-jetons.md) | Pseudonymisation par jetons, aucune donnée nominative hébergée | Acceptée |
| [0002](0002-jetons-stables-a-l-annee.md) | Jetons stables sur l'année scolaire | Acceptée |
| [0003](0003-administration-par-facade-mcp.md) | Administration par façade MCP plutôt qu'interface complète | Acceptée |
| [0004](0004-mot-de-passe-enseignant.md) | Mot de passe fixé pour l'enseignant, passkeys reportées | Acceptée (provisoire) |
| [0005](0005-cle-api-et-choix-du-hachage.md) | Clé d'API pour l'agent, et SHA-256 plutôt qu'Argon2id | Acceptée |
| [0006](0006-serde-norway.md) | `serde_norway` plutôt que `serde_yaml` | Acceptée |
