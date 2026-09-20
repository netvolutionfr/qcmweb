# ADR-0003 — Administration par façade MCP plutôt qu'interface complète

**Statut** : Acceptée — 2026-09-20

## Contexte

Les sujets sont majoritairement rédigés par un agent IA. Coder une interface
d'administration complète — éditeur de sujets, formulaire d'import, formulaire
de création d'évaluation, gestion des groupes — revenait à construire des écrans
dont l'utilisateur principal serait… un agent passant par un navigateur piloté.

## Décision

Le backend expose une **façade MCP** en plus de son API REST, montée dans la
même application Axum (`rmcp`, `StreamableHttpService` comme service Tower),
derrière le même middleware d'authentification et les mêmes autorisations.

Le front enseignant se réduit à quatre écrans : prévisualisation et validation
d'un sujet, génération de jetons et billets, ouverture/fermeture d'une
évaluation, résultats avec jointure locale et export.

La frontière est posée par une règle : **l'agent écrit, l'humain publie.** Toute
transition qui rend un contenu visible aux élèves, ou qui détruit des données,
relève exclusivement du navigateur.

## Conséquences

**Acquis** — la majeure partie de l'interface d'administration n'a pas à être
écrite. La façade est une seconde vue sur des handlers existants, pas un
composant.

**Sécurité** — la règle de partage n'est pas une précaution décorative. Un agent
peut lire un document préparatoire contenant des instructions hostiles. La
protection ne repose pas sur la détection de ces instructions, mais sur le fait
qu'un agent compromis ne peut, au pire, que déposer un brouillon indésirable. La
porte `DRAFT → VALIDATED` humaine est la protection réelle, et ne doit jamais
être automatisée — pas même « pour les évaluations formatives ».

**Frontière RGPD** — aucun outil MCP ne touche aux participants, aux jetons ni à
la table de correspondance. Tout ce qui transite par un outil MCP entre dans le
contexte d'un LLM ; la jointure nominative reste donc navigateur, conformément à
[ADR-0001](0001-pseudonymisation-par-jetons.md).

**Coût** — sans agent sous la main, il n'y a plus d'interface d'administration.
Atténuations : les quatre écrans conservés couvrent le chemin critique en
classe, et l'OpenAPI reste utilisable en `curl`.

## Alternatives écartées

- **Interface d'administration complète** — plusieurs semaines d'écrans pour un
  utilisateur qui préfère écrire du YAML.
- **Pont OpenAPI → MCP générique** — zéro code, mais produit une vingtaine
  d'outils mécaniques, une ergonomie déplorable et un contexte saturé. Huit
  outils taillés pour la tâche valent mieux qu'une transposition automatique.
- **Serveur MCP séparé appelant l'API** — un second processus, un second
  déploiement, une seconde configuration d'authentification, pour aucun gain.

Voir [SPEC.md](../../SPEC.md) §9 et §19.
