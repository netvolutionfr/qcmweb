# ADR-0006 — `serde_norway` plutôt que `serde_yaml`

**Statut** : Acceptée — 2026-09-20

## Contexte

Le format natif `qcm/v1` est édité en YAML, c'est le format que produit un agent
et que relit l'enseignant. Il faut donc un désérialiseur YAML compatible serde.

`serde_yaml` est le choix historique de l'écosystème Rust. Il est **archivé
depuis mars 2024** et ne reçoit plus de correctifs.

## Décision

Utiliser `serde_norway`, fork maintenu de `serde_yaml`.

## Conséquences

**Bénéfice principal** — une dépendance qui reçoit encore des correctifs, sur un
composant qui analyse une entrée non fiable.

**Bénéfice secondaire, et non accessoire** — `serde_norway` corrige le *Norway
problem*. En YAML 1.1, `no` est désérialisé en booléen `false`. Or le format
`qcm/v1` comporte des identifiants de réponse courts, et `id: no` est
exactement le genre d'identifiant qu'un agent produit pour une question
fermée. Avec `serde_yaml`, cet identifiant devenait silencieusement `false` :
une corruption discrète du sujet, découverte au mieux à la relecture, au pire
pendant l'évaluation.

Un test verrouille ce comportement (`identifiant_no_reste_une_chaine`).

## Alternatives écartées

- **`serde_yaml`** — archivé, et porteur du *Norway problem*.
- **`serde_yml`** — fork dont la gouvernance a été contestée dans la communauté.
- **`saphyr`** — activement développé, mais sans intégration serde directe au
  moment du choix ; il faudrait écrire la couche de désérialisation.

À revoir si `saphyr` propose une intégration serde mature, ou si
`serde_norway` cesse à son tour d'être maintenu.
