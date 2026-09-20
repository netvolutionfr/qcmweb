# ADR-0001 — Pseudonymisation par jetons, aucune donnée nominative hébergée

**Statut** : Acceptée — 2026-09-20

## Contexte

L'application traite des résultats d'évaluation d'élèves mineurs. L'objectif
posé était de ne pas avoir à héberger les noms et prénoms, et si possible de
n'avoir ni autorisation à demander, ni traitement à ajouter au registre.

Deux idées ont été examinées puis écartées d'emblée :

- **Déplacer les noms sur le poste de l'enseignant ne supprime pas le
  traitement.** Un tableur nominatif local est un traitement comme un autre.
- **L'exception domestique (art. 2.2.c RGPD) ne s'applique pas.** Un enseignant
  qui évalue ses élèves agit dans le cadre de sa mission professionnelle.

## Décision

Le serveur ne stocke aucune donnée nominative. Il connaît des **participants**,
c'est-à-dire des jetons aléatoires rattachés à un groupe.

La correspondance `jeton → nom, prénom` réside uniquement sur le poste de
l'enseignant. La jointure qui affiche les vrais noms s'exécute dans son
navigateur, en mémoire.

Trois conséquences sont imposées au code :

1. Aucun champ nominatif en base, y compris optionnel ou libre.
2. Aucun endpoint n'accepte de liste d'élèves — la production des billets est
   une opération strictement navigateur.
3. Aucune donnée d'élève ne transite par un outil MCP, donc par un LLM.

## Conséquences

**Acquis** — une compromission du serveur n'expose aucune identité ;
l'hébergeur ne traite aucune donnée nominative ; la minimisation (art. 5.1.c)
devient structurelle et non déclarative ; l'effacement se réduit à la purge d'un
jeton.

**Coût** — l'enseignant doit conserver et sauvegarder sa table de
correspondance. Sa perte rend les résultats définitivement non attribuables.

**Limite à ne pas travestir** — les jetons restent des données à caractère
personnel au sens du considérant 26 : ils sont pseudonymisés, non anonymisés,
puisque l'enseignant détient la clé. Le RGPD s'applique toujours. Cette
conception réduit fortement la surface, elle ne fait pas sortir du cadre.

## Alternatives écartées

- **Noms chiffrés côté serveur, clé sur le poste** — du code cryptographique à
  maintenir, et le chiffré reste hébergé. Plus de complexité pour un gain
  juridique nul.
- **Alias courts stockés côté serveur** (« Élève 07 ») — lisibles sans la table
  locale, mais ils invitent à taper « Alice M. », ce qui ramène au point de
  départ. Une protection qu'on peut contourner par confort n'en est pas une.

Voir [SPEC.md](../../SPEC.md) §4 et §18.
