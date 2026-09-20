# ADR-0005 — Clé d'API pour l'agent, et SHA-256 plutôt qu'Argon2id

**Statut** : Acceptée — 2026-09-20

## Contexte

[ADR-0003](0003-administration-par-facade-mcp.md) donne à un agent un accès
direct au backend. Restait à choisir comment il s'authentifie. Réutiliser le
mot de passe enseignant de [ADR-0004](0004-mot-de-passe-enseignant.md) était
l'option la plus économique à première vue.

## Décision

L'agent s'authentifie par **clé d'API** portée par `Authorization: Bearer`,
distincte du mot de passe enseignant, et porteuse d'un **scope**.

La clé fait 256 bits d'aléa, préfixés `qcmw_`. Elle est créée en ligne de
commande, affichée une seule fois, porte un libellé, et est révocable
individuellement.

**Son empreinte est stockée en SHA-256, et non en Argon2id.**

## Justification du choix de hachage

C'est le point le plus contre-intuitif de cette ADR : le projet hache les mots
de passe en Argon2id, et il serait tentant d'appliquer la même règle partout.

Un KDF coûteux comme Argon2id n'a qu'une fonction : rendre la recherche
exhaustive impraticable sur un secret **à faible entropie**, c'est-à-dire choisi
par un humain. Un mot de passe vit dans un espace que l'on peut parcourir ;
ralentir chaque essai de 100 ms change tout.

Une clé d'API de 256 bits aléatoires ne vit pas dans un tel espace. Aucune
recherche exhaustive n'aboutira, quel que soit le coût unitaire d'un essai.
Argon2id n'apporterait donc rien — mais il coûterait 100 ms de CPU à **chaque
requête d'agent**, alors qu'un agent en enchaîne des dizaines.

La règle générale est donc : **le coût du hachage se choisit selon l'entropie du
secret, pas selon son importance.** Dans les deux cas, la base ne contient
qu'une empreinte : une lecture de la base ne doit jamais suffire à usurper une
identité.

## Conséquences

- Une clé par agent ou par machine : on révoque l'une sans toucher au mot de
  passe de l'enseignant ni déconnecter les autres outils.
- Le scope est porté par le credential. La règle « l'agent écrit, l'humain
  publie » cesse d'être une convention appliquée par les handlers : une clé de
  scope `agent` ne peut structurellement pas valider un sujet.
- Une clé n'étant jamais tapée par un humain, elle peut avoir une entropie
  pleine — c'est précisément ce qui autorise le choix de hachage ci-dessus.
- **Hygiène** : la clé se place dans la configuration du client MCP, jamais dans
  une conversation. Un secret collé dans un prompt part chez un fournisseur de
  modèle et se retrouve dans des journaux.
- `last_used_at` est renseigné à chaque usage : sans valeur de sécurité propre,
  mais permet de repérer une clé oubliée, donc à révoquer.

## Alternatives écartées

- **Réutiliser le mot de passe enseignant** — un agent ne se connecte pas
  interactivement ; il faudrait lui confier le secret principal, sans révocation
  granulaire ni scope. Un agent compromis emporterait alors tous les pouvoirs.
- **Argon2id sur les clés** — 100 ms par requête d'agent pour un gain nul, comme
  démontré plus haut.
- **Clé en clair en base** — une lecture de la base donnerait un accès direct.

Voir [SPEC.md](../../SPEC.md) §9 et §17.
