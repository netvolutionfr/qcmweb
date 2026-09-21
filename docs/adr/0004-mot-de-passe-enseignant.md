# ADR-0004 — Mot de passe fixé pour l'enseignant, passkeys reportées

**Statut** : Acceptée (provisoire) — 2026-09-20. La limitation des tentatives est précisée par [ADR-0015](0015-authentification-durcie.md).
**Cible** : remplacement par WebAuthn/passkeys, ADR à rédiger le moment venu.

## Contexte

L'instance sert un seul enseignant. Les passkeys sont la cible souhaitée, mais
leur mise en œuvre (enregistrement, récupération, gestion multi-appareils) n'est
pas le premier chantier utile.

L'enjeu est mesuré : un seul compte, et aucune donnée nominative derrière ce
compte du fait de [ADR-0001](0001-pseudonymisation-par-jetons.md).

## Décision

Authentification par mot de passe, empreinte **Argon2id** au format PHC placée
en configuration (`TEACHER_PASSWORD_HASH`), jamais en base.

Ni inscription, ni réinitialisation en libre-service, ni courriel de
récupération. Un mot de passe oublié se change en régénérant une empreinte via
`cargo run -- hash-password`.

Session **opaque et côté serveur**, cookie `HttpOnly; Secure; SameSite=Strict`.
La base ne stocke qu'une empreinte du jeton de session.

Limitation des tentatives **par adresse IP**, avec quota de 10 par quart d'heure.

## Conséquences

- Pas de « mot de passe oublié » : c'est cohérent, un tel mécanisme supposerait
  une adresse électronique, soit exactement le type de donnée que l'application
  refuse de détenir.
- Le comptage par IP, et non global, est délibéré : un compteur global
  permettrait à un tiers de verrouiller l'enseignant hors de son propre outil.
- La session est révocable côté serveur, et une lecture de la base ne suffit pas
  à en usurper une.
- `ponytail:` le compteur de tentatives vit en mémoire — remis à zéro au
  redémarrage, non partagé entre instances. Suffisant pour une instance unique.
- Derrière un reverse proxy, toutes les requêtes porteront l'IP du proxy. Lire
  `X-Forwarded-For` le jour où il y en a un, et uniquement s'il est de confiance.

## Alternatives écartées

- **JWT** — pas de révocation sans liste de rejet, donc l'état qu'on prétendait
  éviter, plus une clé de signature à gérer. Une table `sessions` est plus
  simple et plus sûre.
- **Passkeys immédiatement** — la bonne cible, mais pas le premier chantier.
  Décision explicitement provisoire : ne pas empiler d'astuces autour du mot de
  passe en attendant, ce serait du code à jeter.

Voir [SPEC.md](../../SPEC.md) §3 et §17.
