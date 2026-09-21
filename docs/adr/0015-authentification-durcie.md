# ADR-0015 — Limitation des tentatives, révocation des sessions et vérification Argon2

**Statut** : Acceptée — 2026-09-21. Précise la limitation des tentatives décrite
par [ADR-0004](0004-mot-de-passe-enseignant.md).

## Contexte

Une revue de l'authentification a relevé quatre défauts. Chacun a été
**reproduit avant d'être corrigé**, et rejoué après.

| Défaut | Constat reproduit |
|---|---|
| Un compteur unique pour l'enseignant et les élèves, remis à zéro par toute connexion réussie | 45 mots de passe enseignant testés sans blocage, en intercalant 5 connexions élève |
| Réinitialiser un secret ne révoque pas les sessions | session élève toujours valide (`200`) après la réinitialisation |
| Argon2 exécuté dans le handler enseignant | `/api/health` passe de 1 ms à 282 ms sous 224 connexions simultanées |
| Mot de passe accepté quel qu'il soit, saisie visible | — |

## Décision

### La clé d'un compteur est ce que l'on cherche à deviner

Une réussite ne libère que le compteur de la chose dont elle **prouve la
connaissance**. Les deux compteurs ont des clés de types différents, si bien que
le partage d'origine n'est plus exprimable :

- mot de passe enseignant : par adresse (`IpAddr`) ;
- secret d'un participant : par **(jeton, adresse)**.

Le défaut était plus large que rapporté. Séparer enseignant et élèves corrigeait
le contournement décrit, mais laissait le même mécanisme **à l'intérieur** du
parcours élève : un élève A pouvait effacer, en se connectant, le quota qui
protégeait le secret de B. Clé par jeton, et réinitialisation sur ce seul jeton.

Le quota est **réservé avant** la vérification, et non compté après : sinon cent
requêtes simultanées obtiendraient cent essais. Un refus ne prolonge pas la
fenêtre, pour qu'on ne puisse pas repousser son déblocage en martelant.

### Pas de compteur sur les jetons inconnus

Essayé, puis écarté. Par adresse, il aurait permis à **un seul élève de
verrouiller toute la classe** derrière la même adresse hors de l'épreuve, en
envoyant une centaine de jetons bidon. Il ne protège de rien de praticable : un
jeton fait environ 39 bits pour une trentaine de valides, et il identifie sans
authentifier ([ADR-0007](0007-format-des-codes.md)).

### Une session ne survit pas au credential qui l'a produite

- **Participant** : un déclencheur PostgreSQL supprime ses sessions quand
  `secret_hash` ou `active` change. Posé en base plutôt qu'en code, comme
  l'immuabilité des versions : la garantie doit tenir quel que soit le chemin
  (API, outil d'administration, requête tapée à la main). Vérifié par un
  `UPDATE` SQL direct.
- **Enseignant** : chaque session porte une empreinte de l'identifiant et du hash
  du mot de passe. La validation la compare à l'empreinte en vigueur, et l'API
  purge les anciennes au démarrage.

### Argon2 hors de l'exécuteur, à concurrence bornée

`verify_blocking` passe par `spawn_blocking` **et** par un sémaphore plafonné à
`min(cœurs, 4)`. Chaque vérification occupe un cœur et une vingtaine de
mégaoctets : sans borne, un flot de connexions en ouvrirait autant que de
threads disponibles.

### Politique de mot de passe, dans l'outil d'administration seulement

12 caractères au moins (comptés en caractères, non en octets) et cinq distincts,
saisie masquée avec confirmation, et lecture depuis un tube conservée pour le
scriptable. La règle **ne vit pas dans `hash_password`** : cette fonction hache
aussi les secrets de participants, plus courts par conception.

## Conséquences

- Après déploiement, les sessions d'enseignant existantes sont refusées : une
  reconnexion.
- Changer le mot de passe enseignant (`.env`, puis redémarrage) révoque toutes ses
  sessions.
- **Compromis de débit** : 224 connexions simultanées se traitent en 4,1 s au lieu
  de 1,2 s. La santé de l'API, elle, passe de 282 ms à 34 ms. Une classe de trente
  élèves n'est pas concernée.
- **Limite persistante** : le compteur enseignant reste par adresse, donc un
  attaquant disposant de nombreuses adresses le contourne. La parade est la
  robustesse du mot de passe, d'où la politique, puis les passkeys.
- **Nuisance résiduelle acceptée** : un tiers situé derrière la *même* adresse et
  connaissant un jeton peut le verrouiller quinze minutes. Le blocage ne dépasse
  pas cette adresse.
- Les compteurs sont en mémoire : un redémarrage les remet à zéro.

## Alternatives écartées

- **Séparer seulement enseignant et élèves** — corrige le contournement rapporté,
  laisse le même défaut entre élèves.
- **Ne jamais remettre à zéro sur réussite** — une classe entière derrière une
  même adresse cumulerait ses fautes de frappe et se verrouillerait.
- **Compteur global sur l'enseignant** — n'importe qui pourrait l'en verrouiller.
- **Compteur par adresse sur les jetons inconnus** — voir plus haut.
- **Révocation applicative seule** — ne tient pas face à une modification directe
  en base.
