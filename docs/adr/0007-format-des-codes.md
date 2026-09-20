# ADR-0007 — Format et entropie des codes destinés à être recopiés

**Statut** : Acceptée — 2026-09-20

## Contexte

Trois secrets de l'application sont recopiés à la main par un humain, depuis un
billet papier ou un tableau : le **jeton de participation**, le **secret du
participant**, le **code d'évaluation**.

Leur contrainte dominante n'est pas cryptographique mais ergonomique. Un
caractère ambigu ne produit pas une faille : il produit un élève bloqué en début
d'épreuve, et un enseignant qui dépanne au lieu de surveiller.

## Décision

**Alphabet commun de 30 caractères** : `23456789ABCDEFGHJKMNPQRSTVWXYZ`.

Sont exclus `0`/`O`, `1`/`I`/`L` pour l'ambiguïté visuelle, et `U` suivant la
convention Crockford — cela réduit la probabilité qu'un tirage produise un mot
malvenu sur un document distribué à des élèves.

| Code | Longueur | Entropie | Forme |
|---|---|---|---|
| jeton de participation | 8 | ~39 bits | `ZQ93-GSKN` |
| secret du participant | 10 | ~49 bits | `PMQX4-7ME4E` |
| code d'évaluation | 6 | ~29 bits | `K7MP4Q` |

Le tirage utilise un **rejet de la queue de distribution** : un simple
`octet % 30` favoriserait les premiers caractères, puisque 256 n'est pas
divisible par 30. Les octets ≥ 240 sont rejetés, pour un biais nul et un coût de
6 tirages sur 256.

Toute saisie est **normalisée** avant comparaison : passage en majuscules et
suppression de tout ce qui n'est pas alphanumérique. Un élève tape en
minuscules, oublie le tiret, en ajoute un au mauvais endroit, ou colle un tiret
cadratin depuis un traitement de texte. Refuser ces saisies serait une mauvaise
façon de faire respecter un format que nous avons choisi.

## Justification des entropies

Elles diffèrent parce que les trois codes ne protègent pas la même chose.

Le **jeton identifie, il n'authentifie pas**. 39 bits rendent l'énumération
inutile ; la protection réelle vient du secret qui l'accompagne.

Le **secret authentifie**, d'où 49 bits — mais c'est peu au regard d'une clé
d'API. C'est précisément pourquoi il est haché en **Argon2id** et non en
SHA-256 : on est ici dans le cas d'un secret à entropie limitée que le coût du
KDF vient compenser, à l'inverse des clés d'API de
[ADR-0005](0005-cle-api-et-choix-du-hachage.md). Combinés, jeton et secret
représentent 88 bits.

Le **code d'évaluation ne protège rien par lui-même** (SPEC §11). Il est annoncé
à voix haute ou projeté, donc court. 29 bits suffisent à empêcher l'énumération
d'évaluations ouvertes, et l'appartenance au groupe reste vérifiée côté serveur.

## Conséquences

- Une collision de jeton est très improbable mais pas impossible. L'insertion
  utilise `ON CONFLICT DO NOTHING` et **retire un nouveau jeton** en cas de
  collision, jusqu'à cinq fois. Sans cela, la collision serait silencieuse : le
  groupe recevrait un jeton de moins que demandé, et un élève se retrouverait
  sans billet le jour de l'évaluation.
- Les secrets ne sont renvoyés **qu'à la création**, jamais réaffichés. Un
  secret perdu se réinitialise, il ne se retrouve pas.

## Alternatives écartées

- **UUID comme jeton** — 122 bits, mais 36 caractères à recopier dont des
  ambiguïtés. Inutilisable sur un billet papier.
- **Alphabet de 32 caractères** pour un tirage sans rejet — il faudrait
  réintroduire deux caractères ambigus, soit exactement ce que l'alphabet vise
  à éviter, pour économiser six tirages sur 256.
- **Secret identique pour tout un groupe** — plus simple à distribuer, mais
  supprime toute individualisation des tentatives.
