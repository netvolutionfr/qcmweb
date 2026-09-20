# ADR-0002 — Jetons stables sur l'année scolaire

**Statut** : Acceptée — 2026-09-20

## Contexte

[ADR-0001](0001-pseudonymisation-par-jetons.md) pose le modèle par jetons. Reste
à fixer leur durée de vie, qui détermine si le serveur détient un répertoire
durable ou seulement des données éphémères.

L'option la plus stricte était de générer des jetons **par évaluation**, purgés
après export. Le serveur n'aurait alors jamais contenu de répertoire d'élèves :
l'application aurait été l'équivalent d'un paquet de copies sans nom, un outil
de calcul temporaire. C'était la seule voie permettant de plaider qu'aucun
nouveau traitement durable n'est créé.

## Décision

Les jetons sont **stables sur l'année scolaire**. Un même billet sert à toutes
les évaluations du groupe.

## Conséquences

**Acquis** — un seul billet à distribuer par an et par élève. La distribution à
chaque séance était le principal frein d'usage de l'option éphémère : une
friction répétée à chaque cours aurait fini par être contournée.

**Coût assumé** — le serveur détient un répertoire pseudonyme durable. Le RGPD
s'y applique et une ligne au registre du responsable de traitement (le chef
d'établissement, non l'enseignant) reste nécessaire. L'objectif initial « pas de
traitement à déclarer » n'est donc pas atteint, et c'est un choix conscient.

Cela rend deux mécanismes obligatoires plutôt que facultatifs :

- un champ `expires_at` sur chaque participant ;
- une **purge de fin d'année** qui est un vrai chemin de code, tracé, et non un
  nettoyage manuel en base.

## Alternatives écartées

- **Jetons par évaluation, purge automatique** — le plus protecteur, écarté pour
  la friction de distribution décrite ci-dessus.
- **Jetons par évaluation, purge manuelle** — même modèle, mais la conformité
  dépendrait de la discipline de l'enseignant. Une garantie qui repose sur le
  fait de ne pas oublier n'est pas une garantie.

Voir [SPEC.md](../../SPEC.md) §4.
