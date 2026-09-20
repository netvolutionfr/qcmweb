# ADR-0013 — Agrégation des résultats et jointure nominative

**Statut** : Acceptée — 2026-09-20

## Contexte

La SPEC §15 décrit un tableau par élève et une vue par question, exportables.
Plusieurs points n'étaient pas tranchés : quelle copie compte lorsque plusieurs
tentatives sont autorisées, comment faire figurer un absent, et où se fait la
jointure avec les noms.

## Décision

### La meilleure copie compte, la plus précoce départageant

Lorsqu'une évaluation autorise plusieurs tentatives, c'est la **meilleure** qui
figure au tableau, et à score égal la **plus précoce**.

C'est la lecture habituelle d'une évaluation à plusieurs essais : on mesure ce
que l'élève a fini par maîtriser, pas sa première approche. Le nombre de copies
remises reste affiché, pour que le choix soit lisible plutôt qu'implicite.

### Les participants sont lus depuis le groupe, pas depuis les copies

Une jointure partant des tentatives ferait disparaître les absents, qui sont
précisément ceux que l'enseignant cherche. La table part donc des participants
du groupe, et les états `remis` / `en cours` / `absent` s'en déduisent.

### Deux indicateurs par question, jamais un seul

- le **taux de réussite** : part des copies ayant obtenu la totalité des points ;
- le **taux moyen** : moyenne des points obtenus, rapportée au barème.

Ils sont confondus tant que la notation est binaire, et divergent dès qu'une
question est notée partiellement. Une question à 85 % de moyenne et 20 % de
réussite complète ne raconte pas du tout la même histoire qu'une question à
85 % des deux — or c'est en évaluation formative que cette lecture sert le plus
(SPEC §15).

### Deux exports, à deux endroits différents

L'export **par jetons** est produit par le serveur. Il n'a aucun nom à
connaître.

L'export **nominatif** est composé dans le navigateur, après jointure avec la
table de correspondance locale. Le faire produire par le serveur supposerait de
lui transmettre les noms, ce qui viderait
[ADR-0001](0001-pseudonymisation-par-jetons.md) de sa substance pour une simple
commodité.

Les deux portent un **BOM UTF-8** et un séparateur `;`. Sans BOM, un tableur
francophone ouvre le fichier en Latin-1 et affiche « Ã© » à la place des
accents : la différence entre un export exploitable et un export à reprendre à
la main.

### L'interface affiche l'état *et* la disponibilité

Une évaluation peut être `OPEN` tout en étant hors de sa fenêtre planifiée
([ADR-0011](0011-ouverture-et-fenetre.md)). Afficher « ouverte » seule laisserait
croire qu'elle est accessible, alors que les élèves se heurtent à un refus. Le
badge distingue donc « en cours », « ouverte, hors fenêtre » et « fermée ».

## Conséquences

- L'agrégation est une fonction pure, testée sans base : c'est elle qui décide
  quelle copie devient une note.
- Les jetons sans correspondance dans la table chargée sont signalés — table
  d'un autre groupe, ou jetons régénérés depuis.
- La correction manuelle d'une copie et le recalcul après neutralisation d'une
  question (SPEC §13 et §14) ne sont pas implémentés. Le détail par question
  étant conservé sur chaque tentative, ils pourront l'être sans migration.

## Alternatives écartées

- **Retenir la dernière copie** — pénalise l'élève qui retente puis abandonne,
  et transforme une évaluation formative en piège.
- **Moyenner les tentatives** — personne ne raisonne ainsi sur un bulletin.
- **Export nominatif produit par le serveur** — il faudrait lui confier les noms.
- **N'afficher que le taux de réussite** — masque exactement ce que la notation
  partielle cherche à mesurer.
