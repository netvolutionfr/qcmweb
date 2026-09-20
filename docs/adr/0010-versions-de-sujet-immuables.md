# ADR-0010 — Versions de sujet immuables, et répartition des états

**Statut** : Acceptée — 2026-09-20

## Contexte

La SPEC §5 pose trois états — `DRAFT`, `VALIDATED`, `ARCHIVED` — et une
exigence : « modifier aujourd'hui un sujet utilisé trois mois auparavant ne
modifie jamais rétroactivement l'évaluation passée ».

Deux questions en découlaient : où placer ces états, et comment garantir
l'immuabilité.

## Décision

### Les états ne vivent pas au même endroit

`DRAFT` et `VALIDATED` portent sur une **version**, car c'est une version que
l'on relit puis que l'on valide, et qu'un même sujet peut légitimement avoir une
v1 validée et une v2 en brouillon.

`ARCHIVED` porte sur le **sujet** : il le retire de la banque courante sans rien
dire des versions déjà utilisées par des évaluations passées, qui restent
lisibles.

### L'immuabilité est garantie par la base, pas par le code

Un déclencheur PostgreSQL refuse toute modification du document d'une version,
ainsi que tout retour de `VALIDATED` vers `DRAFT`.

Ce choix est délibérément plus lourd qu'une simple discipline applicative. Il
s'agit d'une exigence d'**auditabilité** : une requête d'administration lancée à
la main un soir de correction doit s'y heurter comme le reste du système. Une
garantie qui ne tient qu'à l'absence de chemin de code dans la version actuelle
n'est pas une garantie, c'est une coïncidence.

Vérifié : un `UPDATE` direct en SQL sur le document d'une version, comme un
retour arrière de statut, sont rejetés par la base.

### Un seul point d'entrée pour YAML et JSON

YAML 1.2 étant un sur-ensemble de JSON, `serde_norway` lit les deux. Le corps
des requêtes de dépôt est donc du **texte brut**, sans négociation de type ni
route d'import distincte. La SPEC prévoyait un `POST /api/subjects/import`
séparé : il devient inutile.

### Le schéma qcm/v1 est publié dans l'OpenAPI

Aucune route ne référence `Document` dans sa signature, puisque les documents
transitent en texte brut. Les schémas sont donc déclarés explicitement dans le
document OpenAPI — c'est par là qu'un agent obtient le schéma exigé par la
SPEC §9, sans qu'il faille maintenir un second artefact.

## Conséquences

- Le seul chemin de modification est la création d'une version. Il n'existe
  aucune route de mise à jour d'un document, et il ne doit jamais en exister.
- La validation est réservée à l'extracteur `Teacher` : un agent porteur d'une
  clé de scope `agent` reçoit 401, ce qui rend la règle « l'agent écrit,
  l'humain publie » ([ADR-0003](0003-administration-par-facade-mcp.md))
  impossible à contourner par inadvertance.
- Titre, nombre de questions et barème total sont dénormalisés sur la version,
  pour lister la banque sans désérialiser chaque document. Ils sont figés en
  même temps que le document, donc sans risque de divergence.

## Alternatives écartées

- **Un statut unique sur le sujet** — rendrait impossible la coexistence d'une
  version validée et d'un brouillon en préparation, alors que c'est le cas
  courant : on retouche un sujet pendant que l'ancien reste utilisable.
- **Immuabilité assurée uniquement par le code** — dépend de la vigilance de
  chaque contributeur et ne protège de rien en cas d'intervention manuelle.
- **Éclater les questions en tables relationnelles** — figerait un modèle
  relationnel pour des types de questions encore à venir (SPEC §21).
- **Route d'import distincte par format** — deux chemins à maintenir pour un
  analyseur qui traite déjà les deux.
