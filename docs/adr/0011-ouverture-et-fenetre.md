# ADR-0011 — Ouverture manuelle et fenêtre planifiée, et moment du code

**Statut** : Acceptée — 2026-09-20

## Contexte

La SPEC §10 décrit une évaluation configurée avec une **ouverture** et une
**fermeture**, tandis que le parcours de la SPEC §11 montre un enseignant qui
*ouvre* l'évaluation. Deux mécanismes, donc, qu'il fallait réconcilier.

La SPEC §11 indique par ailleurs que le code est généré « lorsqu'une évaluation
est publiée », alors que [ADR-0003](0003-administration-par-facade-mcp.md)
prévoit qu'un agent crée l'évaluation et **retourne son code** — sans pouvoir la
publier.

## Décision

### Deux conditions cumulatives, volontairement distinctes

Une évaluation n'est composable que si :

- l'enseignant l'a **ouverte** (`state = OPEN`) ;
- **et** l'instant est dans la fenêtre planifiée, quand elle est renseignée.

Ce n'est pas une redondance. Les deux mécanismes répondent à des besoins
différents :

- la fenêtre permet de **préparer à l'avance** sans que l'évaluation devienne
  accessible, et de programmer une fermeture à l'heure dite sans être devant
  son écran ;
- l'état permet de **refermer d'un geste** — un incident, une classe qui
  déborde — sans avoir à recalculer des dates.

La règle tient dans une fonction pure, `is_available`, couverte par des tests
portant notamment sur les bornes : l'ouverture planifiée est inclusive, la
fermeture exclusive. À l'heure dite, c'est fini.

### Le code est généré à la création

Un agent doit pouvoir rendre le code dans la foulée. Il est donc tiré à la
création, alors que l'évaluation naît en `DRAFT` et n'ouvre sur rien.

Cela ne diminue aucune garantie : le code n'a jamais été un moyen
d'authentification (SPEC §11). Le connaître à l'avance ne permet pas de
composer, puisque l'évaluation doit être ouverte, que le participant doit
s'authentifier par jeton et secret, et que son appartenance au groupe est
vérifiée côté serveur.

### La version validée est exigée par la base

Un déclencheur PostgreSQL refuse toute évaluation fondée sur une version qui
n'est pas `VALIDATED`.

C'est la contrainte qui rend défendable l'autonomie laissée à l'agent : il crée
des évaluations, mais uniquement sur des sujets que l'enseignant a relus.
Comme pour l'immuabilité des versions
([ADR-0010](0010-versions-de-sujet-immuables.md)), la garantie est posée là où
elle ne dépend pas du chemin emprunté. Son message est traduit en `409` avec
son texte, plutôt qu'en `500` muet : c'est un refus légitime qui doit se lire.

## Conséquences

- Une évaluation ouverte hors de sa fenêtre affiche `state: OPEN` mais
  `available: false`. L'interface doit montrer les deux, faute de quoi
  l'enseignant croira son évaluation accessible.
- Les aménagements individuels (SPEC §16) ont leur table, rattachée au jeton du
  participant, sans motif nominatif ni mention de santé. Les routes viendront
  avec les tentatives, seul moment où ils produisent un effet.
- Les collisions de code sont traitées par nouveau tirage, comme pour les
  jetons : deux évaluations partageant un code enverraient une classe composer
  le sujet d'une autre.

## Alternatives écartées

- **Seulement des dates** — impose de modifier une configuration pour réagir à
  un imprévu, au pire moment.
- **Seulement un interrupteur** — oblige l'enseignant à être devant son écran à
  l'heure d'ouverture comme à l'heure de fermeture.
- **Code généré à l'ouverture** — empêcherait un agent de le restituer, et
  obligerait l'enseignant à venir le chercher après coup pour le projeter.
