# Plateforme QCM — Spécification fonctionnelle v0.1

## 1. Vision du produit

L'application est une plateforme web légère destinée à la création, la diffusion et la correction d'évaluations principalement constituées de QCM.

Elle doit pouvoir être utilisée aussi bien :

- pour une **évaluation formative**, sans enjeu de notation, avec résultat et correction éventuellement immédiats ;
- pour une **évaluation sommative**, dont le résultat pourra être utilisé comme note ;
- pour des exercices d'auto-évaluation ou de positionnement.

L'application se situe fonctionnellement entre :

- **Kahoot**, pour la simplicité d'accès à une évaluation au moyen d'un code, mais sans gamification, classement ou synchronisation des participants ;
- **Auto Multiple Choice**, pour la possibilité de décrire un questionnaire dans un format texte simple et de définir précisément sa correction ;
- **Quizinière**, pour la logique enseignant → diffusion → réponses → résultats ;
- **Moodle**, pour certaines possibilités d'évaluation, mais sans chercher à devenir un LMS.

L'objectif principal est de pouvoir passer très rapidement de :

> « Voici les objectifs et le contenu de ma séance »

à :

> « Voici un fichier de QCM généré par un agent IA que je peux relire, importer et publier. »

L'IA ne fait pas partie du moteur d'évaluation lui-même. Elle est un outil de **production du sujet**, en amont.

---

# 2. Principes fondamentaux

Trois objets métier doivent être clairement séparés.

### Sujet

Un **sujet** contient les questions, propositions de réponses, bonnes réponses, explications, barème par défaut et métadonnées pédagogiques.

Il appartient à une banque de sujets et peut être réutilisé.

Un sujet est versionné.

### Évaluation

Une **évaluation** correspond à l'utilisation d'une version précise d'un sujet dans des conditions données.

Elle définit notamment :

- le groupe concerné ;
- la date ou période d'ouverture ;
- le nombre de tentatives autorisées ;
- la durée éventuelle ;
- l'ordre aléatoire ou non des questions ;
- l'ordre aléatoire ou non des réponses ;
- les modalités d'affichage des résultats ;
- le caractère formatif ou sommatif ;
- la conversion éventuelle du score en note sur 20.

La création d'une évaluation génère un **code d'accès court**.

### Tentative

Une **tentative** représente le passage d'une évaluation par un élève.

Elle contient :

- l'élève ;
- l'évaluation ;
- l'heure de début ;
- l'heure de remise ;
- les réponses données ;
- le score brut ;
- le score maximal ;
- le pourcentage ;
- éventuellement la note obtenue ;
- la version du barème ayant servi à la correction.

Une tentative commencée doit pouvoir être reprise après une coupure réseau ou la fermeture accidentelle du navigateur.

---

# 3. Utilisateurs

## Enseignant

L'enseignant dispose d'un compte authentifié.

Il peut :

- créer et gérer ses groupes ;
- ajouter ou importer des élèves ;
- créer/importer des sujets ;
- modifier les sujets ;
- consulter leur historique ;
- créer une évaluation ;
- ouvrir et fermer une évaluation ;
- visualiser les tentatives en cours ;
- consulter les résultats ;
- exporter les résultats ;
- éventuellement modifier un barème après l'évaluation ;
- invalider une question problématique et recalculer les résultats.

## Élève

L'élève dispose d'une identité connue à l'avance par le système.

Aucune adresse électronique ne doit être nécessaire.

Il peut :

- s'authentifier ;
- saisir un code d'évaluation ;
- accéder à une évaluation à laquelle son groupe est autorisé ;
- répondre aux questions ;
- reprendre une tentative interrompue ;
- remettre définitivement sa copie ;
- consulter son résultat selon les règles définies par l'enseignant.

Un élève ne peut accéder ni aux copies ni aux résultats des autres élèves.

---

# 4. Gestion des élèves

L'application gère des groupes tels que :

`1SIO`, `2SIO-SLAM`, `2SIO-SISR`, `1NSI`, etc.

Un élève possède au minimum :

```text
id
nom
prenom
identifiant_connexion
secret_authentification
actif
```

L'identifiant interne ne doit pas être construit à partir du nom et du prénom.

Pour éviter l'utilisation d'adresses électroniques ou de comptes tiers, l'application peut générer un identifiant et un secret initial pour chaque élève.

L'enseignant doit pouvoir :

- réinitialiser ce secret ;
- désactiver un élève ;
- transférer un élève vers un autre groupe ;
- importer une liste depuis CSV ;
- exporter la liste ;
- supprimer ou anonymiser les données d'un ancien élève.

Une évolution ultérieure pourra permettre une authentification OIDC, LDAP ou via l'ENT.

---

# 5. Cycle de vie d'un sujet

Un sujet passe par les états suivants :

```text
DRAFT
VALIDATED
ARCHIVED
```

L'import d'un fichier produit systématiquement un brouillon.

L'enseignant doit pouvoir visualiser le questionnaire exactement comme le verra l'élève avant de le valider.

Lorsqu'un sujet validé est modifié, une **nouvelle version** est créée.

Une évaluation référence toujours une version précise et immuable du sujet.

Ainsi, modifier aujourd'hui un sujet utilisé trois mois auparavant ne modifie jamais rétroactivement l'évaluation passée.

---

# 6. Format des sujets

L'application possède son propre modèle de données indépendant des formats externes.

Le même document doit pouvoir être représenté en **YAML ou JSON**.

Le YAML est privilégié comme format d'édition humaine et de génération par les agents IA.

Le JSON est particulièrement adapté :

- aux API ;
- au stockage JSONB PostgreSQL ;
- à JSON Schema ;
- aux tests automatisés.

Exemple :

```yaml
schema: qcm/v1

metadata:
  title: "HTTP et architecture Web"
  description: "Évaluation de fin de séance"
  subject: "BTS SIO"
  tags:
    - http
    - web
    - reseau

questions:

  - id: http-method-get
    type: single_choice
    prompt: |
      Quelle méthode HTTP est normalement utilisée
      pour demander une ressource sans la modifier ?

    choices:
      - id: a
        text: "POST"
      - id: b
        text: "GET"
        correct: true
      - id: c
        text: "DELETE"
      - id: d
        text: "PATCH"

    points: 1

    explanation: |
      GET est une méthode HTTP utilisée pour récupérer
      la représentation d'une ressource.

    objectives:
      - "Identifier les principales méthodes HTTP"

  - id: http-codes
    type: multiple_choice
    prompt: |
      Parmi les codes suivants, lesquels correspondent
      à des erreurs côté client ?

    choices:
      - id: a
        text: "200"
      - id: b
        text: "404"
        correct: true
      - id: c
        text: "403"
        correct: true
      - id: d
        text: "500"

    scoring:
      mode: exact

    points: 2

    explanation: |
      Les codes HTTP de la famille 4xx correspondent
      aux erreurs associées à la requête du client.
```

Le texte des questions et réponses accepte **Markdown**.

C'est particulièrement important pour l'enseignement de l'informatique afin de permettre :

```python
for i in range(10):
    print(i)
```

avec coloration syntaxique du code.

---

# 7. Types de questions

Pour le MVP, seuls quelques types sont nécessaires.

### `single_choice`

Une et une seule réponse correcte.

### `multiple_choice`

Plusieurs réponses peuvent être correctes.

Le moteur doit au minimum proposer deux modes de notation :

`exact`

Le point n'est obtenu que lorsque l'ensemble exact des réponses correctes est sélectionné.

`partial`

Une partie des points peut être obtenue selon les choix corrects et incorrects.

Le score minimal d'une question est par défaut zéro.

### `true_false`

Cas simplifié de `single_choice`.

Les types suivants sont volontairement reportés à une version ultérieure :

- réponse numérique ;
- réponse textuelle courte ;
- expression régulière ;
- association ;
- classement ;
- texte à trous ;
- réponse libre corrigée par l'enseignant ;
- exécution ou analyse automatique de code.

---

# 8. Import et export

Le format `qcm/v1` constitue le format natif.

Des adaptateurs pourront importer d'autres formats.

Le premier adaptateur externe intéressant est **AMC-TXT**.

L'import suit donc cette architecture :

```text
AMC-TXT ──────┐
              │
YAML qcm/v1 ──┼──> modèle interne qcm/v1
              │
JSON qcm/v1 ──┘
```

Le format interne ne doit surtout pas dépendre directement d'AMC.

Cela permettra ultérieurement d'ajouter :

```text
Moodle XML
GIFT
QTI
CSV
etc.
```

sans modifier le cœur de l'application.

---

# 9. Génération par IA

Le workflow privilégié est :

```text
Cours / objectifs pédagogiques
           ↓
       agent IA
           ↓
       YAML qcm/v1
           ↓
 validation syntaxique
           ↓
 validation fonctionnelle
           ↓
 relecture enseignant
           ↓
       publication
```

L'application doit fournir un JSON Schema ou un schéma équivalent permettant à un agent de vérifier son résultat.

Des validations métier doivent également être effectuées.

Par exemple :

```text
single_choice → exactement une réponse correcte
multiple_choice → au moins une réponse correcte
points > 0
ID de question unique
ID de réponse unique dans la question
```

L'IA ne participe pas à la correction pendant l'évaluation.

Les données nominatives des élèves n'ont donc aucune raison d'être envoyées à un LLM.

---

# 10. Création d'une évaluation

À partir d'un sujet, l'enseignant choisit :

```text
Nom de l'évaluation
Groupe
Mode : FORMATIVE / SUMMATIVE
Ouverture
Fermeture
Durée éventuelle
Nombre de tentatives
Mélange des questions
Mélange des propositions
Affichage du score
Affichage de la correction
Barème final
```

Les résultats peuvent être configurés selon plusieurs stratégies :

```text
score:
  IMMEDIATE
  AFTER_CLOSE
  NEVER

correction:
  IMMEDIATE
  AFTER_CLOSE
  NEVER
```

Ainsi une évaluation formative peut avoir :

```text
score = IMMEDIATE
correction = IMMEDIATE
```

alors qu'une évaluation sommative peut utiliser :

```text
score = IMMEDIATE
correction = AFTER_CLOSE
```

pour éviter qu'un étudiant ayant terminé rapidement transmette les réponses aux autres.

---

# 11. Code d'évaluation

Lorsqu'une évaluation est publiée, un code court est généré, par exemple :

```text
K7MP4Q
```

Les caractères ambigus (`0/O`, `1/I`, etc.) peuvent être exclus.

Le code :

- n'est pas un moyen d'authentification ;
- ne révèle aucune information ;
- doit être suffisamment aléatoire pour empêcher l'énumération.

Le workflow élève devient alors :

```text
connexion
   ↓
code K7MP4Q
   ↓
vérification de l'appartenance au groupe
   ↓
présentation des consignes
   ↓
Démarrer
   ↓
questionnaire
   ↓
Remettre définitivement
   ↓
résultat éventuel
```

---

# 12. Passage de l'évaluation

Les questions sont obtenues depuis le serveur sans jamais transmettre les propriétés :

```text
correct
explanation
barème détaillé
```

avant que leur divulgation soit autorisée.

Il ne doit donc pas être possible de trouver les réponses simplement en ouvrant les outils de développement du navigateur.

Chaque réponse est sauvegardée automatiquement.

Le serveur reste la référence pour :

- l'heure de début ;
- la durée ;
- l'heure de remise ;
- le calcul du score.

Un stockage local du navigateur peut conserver temporairement les réponses en cas de coupure réseau.

---

# 13. Correction

La correction est entièrement réalisée côté serveur.

Le système conserve toujours :

```text
points obtenus
points maximum
pourcentage
note convertie
```

Une note sur 20 n'est donc qu'une représentation du score :

```text
note = score / score_max × 20
```

avec éventuellement une règle d'arrondi configurable.

L'enseignant peut modifier le résultat d'une tentative.

Toute modification manuelle d'une évaluation sommative doit être tracée.

---

# 14. Recalcul d'une évaluation

Cette fonctionnalité est importante dans un contexte réel d'enseignement.

Si, après l'évaluation, l'enseignant constate qu'une question était ambiguë ou erronée, il peut par exemple :

```text
Question 7
→ neutraliser la question
```

Le moteur recalcule alors toutes les copies.

L'historique conserve :

```text
barème initial
barème modifié
date de modification
enseignant
motif éventuel
```

Le résultat est donc reproductible et auditable.

---

# 15. Tableau de résultats

L'enseignant dispose d'une vue synthétique :

| Élève | État | Score | % | Note | Durée |
|---|---|---:|---:|---:|---:|
| Alice Martin | remis | 17/20 | 85 % | 17 | 12:14 |
| Bob Dupont | remis | 13/20 | 65 % | 13 | 18:42 |
| Charlie Durand | en cours | — | — | — | — |
| David Bernard | absent | — | — | — | — |

Une vue par question permet également d'obtenir :

```text
Q1 : 92 % de réussite
Q2 : 81 %
Q3 : 37 %
Q4 : 74 %
```

Cette information est particulièrement utile en évaluation formative puisqu'elle permet d'identifier les notions insuffisamment maîtrisées par le groupe.

Les résultats peuvent être exportés au minimum en :

```text
CSV
JSON
```

---

# 16. Aménagements individuels

Une évaluation peut contenir des paramètres spécifiques à certains élèves.

Par exemple :

```text
temps supplémentaire
tentative supplémentaire
absence justifiée
nouvelle tentative autorisée
```

Cette possibilité doit être prévue dans le modèle même si l'interface complète peut arriver ultérieurement.

---

# 17. Sécurité

Le backend constitue l'unique source de vérité.

Les principales règles sont :

- authentification obligatoire ;
- autorisation systématique côté serveur ;
- aucune bonne réponse envoyée prématurément au navigateur ;
- mots de passe ou secrets stockés avec Argon2id ;
- limitation des tentatives de connexion ;
- HTTPS obligatoire ;
- identifiants aléatoires non séquentiels exposés à l'extérieur ;
- contrôles empêchant les accès horizontaux entre élèves ;
- contrôle strict de l'appartenance aux groupes ;
- journalisation des opérations sensibles ;
- sauvegarde de PostgreSQL.

Le système ne cherche pas à mettre en place une surveillance intrusive de type « proctoring ».

La détection de changement d'onglet ou de sortie du plein écran ne doit notamment jamais être considérée comme une preuve de fraude.

Les mécanismes raisonnables sont plutôt :

```text
mélange des réponses
mélange des questions
durée limitée
une seule tentative
questions tirées éventuellement d'un pool
```

---

# 18. Données personnelles

Les données minimales concernant un élève sont :

```text
nom
prénom
groupe
identifiant
résultats aux évaluations
```

Aucune donnée telle que :

```text
adresse
téléphone
email personnel
date de naissance
```

n'est nécessaire.

L'application doit permettre de définir une politique de conservation et de supprimer ou anonymiser les données des élèves.

Elle ne doit embarquer aucun tracker publicitaire ou outil d'analyse tiers par défaut.

La question de la base légale RGPD dépend du responsable du traitement et du contexte d'utilisation ; elle ne doit pas être figée dans les spécifications techniques de l'application.

---

# 19. Architecture technique proposée

```text
┌─────────────────────────┐
│       Navigateur        │
│                         │
│ Next.js / TypeScript    │
│ enseignant + élève      │
└────────────┬────────────┘
             │ HTTPS / JSON
             │
┌────────────▼────────────┐
│        API Rust         │
│                         │
│ Axum                    │
│ Tokio                   │
│ Serde                   │
│ SQLx                    │
│ OpenAPI                 │
└────────────┬────────────┘
             │
┌────────────▼────────────┐
│      PostgreSQL         │
└─────────────────────────┘
```

Le tout est déployable avec Docker Compose.

Éventuellement :

```text
Nginx
  ↓
frontend

/api/*
  ↓
API Rust

PostgreSQL
```

Un stockage objet compatible S3 pourra être ajouté ultérieurement pour les images intégrées aux questions.

---

# 20. Choix de Rust

Le projet se prête relativement bien à Rust.

Le backend est principalement constitué de :

```text
API REST
validation
authentification
machine à états
règles de correction
accès PostgreSQL
```

Il n'a pas besoin d'un framework applicatif extrêmement riche.

Une stack :

```text
Axum
Tokio
Serde
SQLx
PostgreSQL
utoipa / OpenAPI
```

est suffisante.

Rust apporte notamment un typage intéressant pour représenter les différents types de questions :

```rust
enum Question {
    SingleChoice(SingleChoiceQuestion),
    MultipleChoice(MultipleChoiceQuestion),
    TrueFalse(TrueFalseQuestion),
}
```

et évite une grande catégorie d'états incohérents.

Pour un développement fortement assisté par agents, le compilateur et les tests constituent également un excellent mécanisme de validation du code produit.

---

# 21. Modèle de données conceptuel

Le premier modèle peut s'articuler autour des entités :

```text
Workspace
User

Group
Student
GroupMembership

Subject
SubjectVersion

Assessment
AssessmentStudentOverride

Attempt
AttemptAnswer

GradingRevision

AuditEvent
```

La relation principale devient :

```text
Subject
   │
   └── SubjectVersion
           │
           └── Assessment
                   │
                   ├── Group
                   │
                   └── Attempt
                           │
                           ├── Student
                           └── AttemptAnswer
```

Les questions du sujet peuvent initialement être conservées sous forme de document JSONB dans `SubjectVersion`.

Cela évite de transformer immédiatement chaque détail d'une question en une multitude de tables relationnelles.

---

# 22. API indicative

L'API pourrait initialement proposer :

```text
POST   /api/auth/login

GET    /api/groups
POST   /api/groups

GET    /api/students
POST   /api/students
POST   /api/students/import

GET    /api/subjects
POST   /api/subjects
POST   /api/subjects/import
GET    /api/subjects/{id}/versions

POST   /api/assessments
GET    /api/assessments/{id}
POST   /api/assessments/{id}/open
POST   /api/assessments/{id}/close

POST   /api/join/{code}

POST   /api/attempts/{id}/start
PUT    /api/attempts/{id}/answers/{questionId}
POST   /api/attempts/{id}/submit

GET    /api/assessments/{id}/results
GET    /api/attempts/{id}/result
```

OpenAPI doit être généré directement depuis le backend Rust afin que le client TypeScript puisse disposer de types générés.

---

# 23. MVP

Le MVP est volontairement limité.

Il doit permettre de réaliser l'intégralité du scénario suivant :

```text
1. L'enseignant crée un groupe.

2. Il importe ses élèves depuis un CSV.

3. Un agent IA génère un qcm/v1 YAML.

4. L'enseignant importe ce fichier.

5. L'application valide le document.

6. L'enseignant prévisualise et valide le sujet.

7. Il crée une évaluation pour son groupe.

8. L'application lui fournit un code.

9. Les élèves se connectent.

10. Ils entrent le code.

11. Ils répondent individuellement au QCM.

12. Les réponses sont sauvegardées automatiquement.

13. Ils remettent leur copie.

14. Le backend calcule le résultat.

15. L'élève obtient le retour prévu par l'enseignant.

16. L'enseignant consulte les résultats individuels
    et les statistiques par question.

17. L'enseignant exporte les résultats en CSV.
```

Tout ce qui n'est pas nécessaire à ce scénario doit être considéré avec prudence avant d'entrer dans le MVP.

---

# 24. Hors périmètre initial

Ne font notamment pas partie du MVP :

```text
classement entre élèves
points d'expérience
badges
chat
visioconférence
cours et ressources pédagogiques
devoirs génériques
cahier de textes
application mobile native
notifications push
marketplace publique de QCM
correction de code par sandbox
surveillance vidéo
intégration Pronote / ÉcoleDirecte
analyse des réponses par IA
```

Le produit doit rester une application d'évaluation et non devenir progressivement un LMS.

---

# 25. Principe produit

Une règle peut résumer l'ensemble :

> En moins de cinq minutes, un enseignant disposant d'un fichier YAML correctement construit doit pouvoir le transformer en une évaluation accessible à ses élèves.

Et côté élève :

> Connexion → code → consignes → questionnaire → remise → résultat.

Tout écran ou toute fonctionnalité qui complique inutilement ces deux parcours doit pouvoir être remis en question.
