# ADR-0012 — Tentatives : le serveur comme seule source de vérité

**Statut** : Acceptée — 2026-09-20

## Contexte

La SPEC §12 pose plusieurs exigences simultanées : les bonnes réponses ne
doivent jamais atteindre le navigateur avant l'heure, une tentative doit pouvoir
être reprise après une coupure réseau, un élève ne doit accéder ni aux copies ni
aux résultats des autres, et le serveur reste la référence pour le temps comme
pour le score.

## Décision

### La fuite des réponses est rendue impossible à écrire

La vue transmise à un élève est produite par des types qui **ne possèdent pas de
champ** `correct`, `explanation` ni de barème détaillé (`domain/exam.rs`).

Ce n'est pas une omission de sérialisation que l'on pourrait rétablir par
mégarde en ajoutant un `#[serde(skip)]` mal placé ou en réutilisant le type du
domaine : l'information n'existe pas dans la structure envoyée. Un test vérifie
l'absence des chaînes correspondantes dans la charge utile sérialisée.

C'est la différence entre « nous n'envoyons pas les réponses » et « nous ne
pouvons pas les envoyer ».

### Le mélange est déterministe, pas aléatoire à chaque appel

Une graine est tirée à la création de la tentative et conservée. L'ordre des
questions et des propositions en est déduit à chaque affichage.

Une tentative reprise après une coupure retrouve donc exactement le même ordre.
Un mélange retiré à chaque requête rendrait toute reprise déroutante, et
l'enregistrement au fil de l'eau incohérent.

Fisher-Yates est écrit à la main plutôt qu'emprunté à `rand::seq` : le mélange
doit rester identique d'une version de dépendance à l'autre, sans quoi une mise
à jour en cours d'année changerait l'ordre d'une copie en cours.

### Reprendre ne relance pas le chronomètre

Redémarrer une tentative en cours renvoie la même tentative, la même graine et
la **même échéance**. Recalculer l'échéance à chaque reprise offrirait un temps
illimité à qui rafraîchit sa page.

L'échéance est calculée côté serveur à partir de la durée de l'évaluation et des
minutes supplémentaires accordées par un aménagement (SPEC §16).

### Le cloisonnement est dans la requête, pas dans un contrôle qui suit

Une tentative est chargée par `WHERE id = $1 AND participant_id = $2`. Il
n'existe aucun chemin qui lise une tentative puis vérifie ensuite son
propriétaire : l'oubli de ce second contrôle est la façon habituelle dont
apparaissent les accès horizontaux.

De même, l'appartenance au groupe est lue sur la session, jamais sur un champ
fourni par le client.

### Une copie remise est figée en base

Un déclencheur PostgreSQL refuse toute écriture de réponse sur une tentative
remise. « Remettre définitivement » doit vouloir dire définitivement, y compris
face à une requête tardive qu'un contrôle applicatif aurait laissé passer.

### Les secrets sont hachés sous leur forme canonique

Le tiret d'un secret (`PMQX4-7ME4E`) n'existe que pour la lisibilité du billet
papier. C'est la forme canonique — majuscules, sans séparateur — qui est hachée
et comparée.

Hacher la forme affichée rendait invalide toute saisie normalisée : le défaut a
été trouvé en jouant le scénario complet, pas en relisant le code. Il précise
[ADR-0007](0007-format-des-codes.md), qui imposait la normalisation des saisies
sans dire de quel côté elle s'applique.

### Un code fermé et un code inexistant répondent la même chose

Les deux renvoient `404`. Distinguer les deux confirmerait l'existence d'un code
et inviterait à l'énumération.

## Conséquences

- Les sessions d'enseignant et de participant partagent une table, discriminées
  par `participant_id`. La clause `participant_id IS NULL` de l'extracteur
  `Teacher` n'est pas un détail : sans elle, la session d'un élève ouvrirait les
  routes enseignantes. Vérifié dans les deux sens.
- La correction s'effectue sur le document **figé** de la version référencée par
  l'évaluation, et son détail est conservé sur la tentative. Le résultat reste
  donc reconstituable même si le sujet évolue ensuite.
- Le recalcul après neutralisation d'une question (SPEC §14) n'est pas encore
  implémenté. Le détail par question étant conservé, il pourra l'être sans
  migration.

## Alternatives écartées

- **Masquer les réponses à l'affichage** — il suffirait d'ouvrir les outils de
  développement. La SPEC l'exclut explicitement.
- **Réutiliser le type du domaine en retirant des champs à la sérialisation** —
  fonctionne jusqu'au jour où quelqu'un ajoute un champ sans y penser.
- **Conserver l'ordre mélangé en base** — une colonne de plus, alors qu'une
  graine et une fonction pure donnent le même résultat et se testent sans base.
- **Faire confiance à l'horodatage du navigateur** — l'horloge du client n'est
  pas une source de vérité, et la SPEC §12 le dit.
