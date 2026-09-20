# Plateforme QCM — Spécification fonctionnelle v0.3

> v0.3 — administration par agent via une façade MCP montée dans le backend.
> Sections 1, 5, 9, 17, 19, 22 et 23 révisées.
>
> v0.2 — passage au modèle par jetons : le serveur ne détient plus aucune donnée
> nominative. Sections 3, 4, 11, 15, 16, 18, 21, 22 et 23 révisées.

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

> « Voici un QCM déposé par un agent IA que je n'ai plus qu'à relire et publier. »

L'IA ne fait pas partie du moteur d'évaluation lui-même. Elle est un outil de
**production du sujet**, en amont, et un moyen d'**administrer l'application**
sans en coder toute l'interface (voir section 9).

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

Une **tentative** représente le passage d'une évaluation par un participant.

Elle contient :

- le participant ;
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
- générer des jetons de participation pour un groupe ;
- produire localement les billets à distribuer ;
- réinitialiser le secret d'un jeton ou le désactiver ;
- créer/importer des sujets ;
- modifier les sujets ;
- consulter leur historique ;
- créer une évaluation ;
- ouvrir et fermer une évaluation ;
- visualiser les tentatives en cours ;
- consulter les résultats ;
- exporter les résultats ;
- éventuellement modifier un barème après l'évaluation ;
- invalider une question problématique et recalculer les résultats ;
- purger un groupe ou une évaluation.

L'enseignant est le seul à connaître la correspondance entre un jeton et un
élève. Cette correspondance ne réside jamais sur le serveur.

## Élève (participant)

Le serveur ne connaît pas d'élève. Il connaît un **participant**, c'est-à-dire
un jeton rattaché à un groupe.

Aucun nom, prénom, adresse électronique ni compte tiers n'est nécessaire.

Le participant peut :

- s'authentifier avec son jeton et son secret ;
- saisir un code d'évaluation ;
- accéder à une évaluation à laquelle son groupe est autorisé ;
- répondre aux questions ;
- reprendre une tentative interrompue ;
- remettre définitivement sa copie ;
- consulter son résultat selon les règles définies par l'enseignant.

Un participant ne peut accéder ni aux copies ni aux résultats des autres.

---

# 4. Gestion des participants

L'application gère des groupes tels que :

`1SIO`, `2SIO-SLAM`, `2SIO-SISR`, `1NSI`, etc.

Un groupe n'est qu'une étiquette et un effectif. Il ne contient aucune liste
nominative.

Un participant possède exactement :

```text
id
jeton
secret_hash
group_id
actif
expire_le
```

Il n'existe **aucun** champ nom, prénom, adresse ou date de naissance, et aucun
moyen d'en ajouter un. Le jeton n'est dérivé d'aucune donnée personnelle : il
est tiré aléatoirement.

## Création des jetons

L'enseignant indique un groupe et un effectif. Le serveur tire N jetons et N
secrets initiaux, et ne retourne les secrets en clair qu'une seule fois.

## Production des billets

La jointure entre les jetons et les élèves réels est réalisée **dans le
navigateur de l'enseignant** :

```text
liste nominative locale (CSV)   jetons renvoyés par le serveur
             │                              │
             └──────────────┬───────────────┘
                            │ jointure en mémoire, côté client
                            ▼
                  billets imprimables
             (nom, prénom, jeton, secret)
```

Le fichier nominatif n'est jamais transmis au serveur. Aucun point d'entrée
d'API n'accepte de liste d'élèves.

L'enseignant conserve sur son poste la table de correspondance
`jeton → nom, prénom`. Elle relève de sa responsabilité, et sa perte rend les
résultats définitivement non attribuables. Sa sauvegarde n'est pas optionnelle.

## Cycle de vie

Les jetons sont **stables sur l'année scolaire** : un même billet sert à toutes
les évaluations du groupe.

L'enseignant doit pouvoir :

- réinitialiser le secret d'un jeton ;
- désactiver un jeton ;
- transférer un jeton vers un autre groupe ;
- purger un groupe en fin d'année.

La purge d'un groupe détruit les jetons et les tentatives associées. C'est une
opération prévue et tracée, et non un nettoyage manuel en base.

Une évolution ultérieure pourra permettre une authentification OIDC, LDAP ou
via l'ENT. Elle réintroduirait des identités sur le serveur et devrait donc
faire l'objet d'une nouvelle analyse au regard de la section 18.

---

# 5. Cycle de vie d'un sujet

Un sujet passe par les états suivants :

```text
DRAFT
VALIDATED
ARCHIVED
```

Tout dépôt de sujet produit systématiquement un brouillon, quelle que soit son
origine.

L'enseignant doit pouvoir visualiser le questionnaire exactement comme le verra
l'élève avant de le valider.

La transition `DRAFT → VALIDATED` est **réservée à l'enseignant**, dans son
navigateur. Elle n'est accessible ni à un agent, ni à aucun automate : c'est la
porte de relecture humaine, et elle constitue à ce titre une garantie de
sécurité autant qu'une garantie pédagogique.

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

# 9. Génération et administration par agent

Les sujets étant le plus souvent produits par un agent IA, l'application lui
donne un accès direct au backend plutôt que de reproduire cet accès dans une
interface d'administration.

Le workflow privilégié est :

```text
Cours / objectifs pédagogiques
           ↓
       agent IA
           ↓
       YAML qcm/v1
           ↓
 validation syntaxique      ─┐
           ↓                 │  boucle autonome de l'agent
 validation fonctionnelle   ─┘
           ↓
 dépôt en brouillon
           ↓
 relecture enseignant        ← navigateur, humain
           ↓
       validation
```

L'application fournit un JSON Schema permettant à un agent de vérifier son
résultat avant tout envoi.

Des validations métier sont également effectuées côté serveur :

```text
single_choice → exactement une réponse correcte
multiple_choice → au moins une réponse correcte
points > 0
ID de question unique
ID de réponse unique dans la question
```

## Façade MCP

Le backend expose une façade **MCP** (Model Context Protocol) en plus de son API
REST. Il ne s'agit pas d'un second service : la façade est montée dans la même
application, derrière le même middleware d'authentification et les mêmes règles
d'autorisation. Elle n'ouvre aucun chemin d'accès qui n'existe pas déjà.

Outils exposés :

```text
qcm_validate_subject(yaml)     dry-run : schéma + règles métier, aucune écriture
qcm_publish_draft(yaml)        crée un DRAFT, retourne l'URL de relecture
qcm_list_subjects
qcm_get_subject(id)
qcm_create_assessment(...)     exige une version validée, retourne le code
qcm_list_assessments
qcm_get_results(id)            pseudonymisé
qcm_question_stats(id)         agrégé
```

Ressource exposée :

```text
qcm://schema/v1                le JSON Schema du format natif
```

Le couple `qcm://schema/v1` + `qcm_validate_subject` ferme la boucle de
génération : l'agent produit, valide, corrige et redépose sans intervention.
`qcm_publish_draft` retourne l'URL de relecture, ce qui donne la jonction
naturelle entre le travail de l'agent et celui de l'enseignant.

## Règle de partage : l'agent écrit, l'humain publie

Toute transition qui rend quelque chose visible aux élèves, ou qui détruit des
données, relève exclusivement du navigateur.

| Agent (MCP) | Enseignant (navigateur) |
|---|---|
| valider un YAML | `DRAFT → VALIDATED` |
| créer un sujet en `DRAFT` | ouvrir / fermer une évaluation |
| lire sujets et versions | générer des jetons, réinitialiser un secret |
| créer une évaluation | purger un groupe |
| lire résultats pseudonymisés | jointure nominative, billets, export nominatif |

La création d'une évaluation est confiée à l'agent : elle ne peut référencer
qu'une version déjà validée par l'enseignant, et l'évaluation reste fermée
jusqu'à son ouverture manuelle. Le risque est donc borné.

Cette règle se traduit par un **scope `agent`** porté par le jeton d'API et
vérifié par le middleware. Aucun outil MCP ne touche aux participants, aux
jetons ni à la purge.

## Limites

L'agent n'accède à aucune donnée nominative, pour une raison structurelle : tout
ce qui transite par un outil MCP entre dans le contexte d'un LLM. La table de
correspondance `jeton → nom, prénom` et la production des billets restent donc
des opérations strictement navigateur (voir section 18).

L'IA ne participe pas non plus à la correction pendant l'évaluation, ni à
l'analyse des réponses.

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

Les caractères ambigus (`0/O`, `1/I`, etc.) sont exclus, aussi bien des codes
d'évaluation que des jetons.

Deux objets distincts coexistent :

```text
jeton + secret   →  authentifie un participant      (annuel, personnel)
code évaluation  →  désigne une évaluation ouverte  (éphémère, collectif)
```

Le code d'évaluation :

- n'est pas un moyen d'authentification ;
- ne révèle aucune information ;
- doit être suffisamment aléatoire pour empêcher l'énumération.

Il joue en revanche le rôle de second facteur contextuel : connaître un jeton
ne suffit pas à composer, encore faut-il que l'évaluation soit ouverte et que
le groupe du jeton y soit autorisé.

Le workflow participant devient :

```text
connexion par jeton + secret
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

Un stockage local du navigateur peut conserver temporairement les réponses en
cas de coupure réseau. Il ne doit contenir que des réponses et le jeton en
cours, jamais de donnée nominative.

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

L'enseignant dispose d'une vue synthétique. Le serveur ne connaissant que des
jetons, c'est ce qu'il affiche :

| Participant | État | Score | % | Note | Durée |
|---|---|---:|---:|---:|---:|
| 7K4M-P2QF | remis | 17/20 | 85 % | 17 | 12:14 |
| B9XT-RM3D | remis | 13/20 | 65 % | 13 | 18:42 |
| Q3FP-K8TW | en cours | — | — | — | — |
| M2RD-X7BN | absent | — | — | — | — |

Une commande « Charger ma liste » permet d'ouvrir la table de correspondance
locale. La jointure s'effectue en mémoire dans le navigateur, le tableau
affiche alors les noms, et cet affichage disparaît au rechargement de la page.
Rien n'est transmis au serveur.

Une vue par question permet également d'obtenir :

```text
Q1 : 92 % de réussite
Q2 : 81 %
Q3 : 37 %
Q4 : 74 %
```

Cette information est particulièrement utile en évaluation formative puisqu'elle
permet d'identifier les notions insuffisamment maîtrisées par le groupe. Elle
est de surcroît entièrement agrégée, donc dépourvue de lien avec un participant.

Les résultats peuvent être exportés au minimum en :

```text
CSV
JSON
```

L'export produit par le serveur est toujours pseudonymisé. Un export nominatif
peut être généré côté client, après jointure locale, en vue d'un report dans le
logiciel de notes de l'établissement.

---

# 16. Aménagements individuels

Une évaluation peut contenir des paramètres spécifiques à certains
participants, désignés par leur jeton :

```text
temps supplémentaire
tentative supplémentaire
absence justifiée
nouvelle tentative autorisée
```

L'enseignant sait à quel élève correspond le jeton ; le serveur, non. Un
aménagement ne comporte donc jamais de motif nominatif, ni aucune mention
relevant de la santé ou d'un diagnostic.

Cette possibilité doit être prévue dans le modèle même si l'interface complète
peut arriver ultérieurement.

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
- sauvegarde de PostgreSQL ;
- façade MCP soumise aux mêmes autorisations que l'API REST ;
- scope restreint pour les jetons d'agent, excluant participants et purge ;
- transitions rendant un contenu visible aux élèves réservées à l'humain ;
- aucun champ nominatif en base, y compris optionnel ou libre ;
- purge effective des jetons et tentatives à l'échéance.

L'exposition d'une façade MCP introduit un vecteur propre : un agent peut lire un
document préparatoire contenant des instructions hostiles. La protection ne
repose pas sur la détection de ces instructions, mais sur le partage des
pouvoirs décrit en section 9 : un agent compromis ne peut, au pire, que déposer
un brouillon indésirable.

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

L'application est conçue pour qu'aucune donnée nominative ne soit hébergée.

Le serveur détient, pour un participant :

```text
jeton
groupe
résultats aux évaluations
```

Il ne détient pas, et ne doit jamais pouvoir détenir :

```text
nom
prénom
adresse électronique
adresse postale
téléphone
date de naissance
```

La correspondance entre un jeton et un élève réside uniquement sur le poste de
l'enseignant, dans le même périmètre que son cahier de notes.

## Portée exacte de cette conception

Les jetons demeurent des données à caractère personnel au sens du considérant 26
du RGPD : ils sont pseudonymisés, non anonymisés, puisque l'enseignant détient
la clé de réidentification. Le traitement reste donc soumis au RGPD et doit
figurer au registre du responsable de traitement — lequel est le chef
d'établissement, et non l'enseignant.

Ce que cette conception apporte réellement :

- une compromission du serveur n'expose aucune identité ;
- l'hébergeur ne traite aucune donnée nominative ;
- la minimisation (art. 5.1.c) devient structurelle et non déclarative ;
- le droit à l'effacement (art. 17) se réduit à la purge d'un jeton ;
- la ligne à porter au registre décrit un traitement minimal.

## Conservation

Les jetons sont valables une année scolaire. Une purge de fin d'année détruit
jetons et tentatives.

Les évaluations passées peuvent être conservées sous forme agrégée
(statistiques par question), dépourvue de tout lien avec un participant.

L'application n'embarque aucun tracker publicitaire ni outil d'analyse tiers, et
n'émet aucune requête vers un domaine tiers depuis le navigateur.

## Limite d'usage

Cette analyse vaut pour une instance exploitée par un enseignant pour ses
propres classes. Ouvrir l'instance à d'autres enseignants modifie la
qualification des responsabilités et impose de la reprendre.

---

# 19. Architecture technique proposée

```text
┌─────────────────────────┐     ┌─────────────────────────┐
│       Navigateur        │     │        Agent IA         │
│                         │     │                         │
│ Next.js / TypeScript    │     │ client MCP              │
│ relecture, billets,     │     │ rédaction des sujets    │
│ ouverture, résultats    │     │                         │
└────────────┬────────────┘     └────────────┬────────────┘
             │ HTTPS / JSON                  │ HTTPS / MCP
             │ /api/*                        │ /mcp
             └───────────────┬───────────────┘
                             │
             ┌───────────────▼───────────────┐
             │           API Rust            │
             │                               │
             │ Axum · Tokio · Serde · SQLx   │
             │ utoipa (OpenAPI) · rmcp (MCP) │
             │                               │
             │ auth et autorisations         │
             │ communes aux deux façades     │
             └───────────────┬───────────────┘
                             │
             ┌───────────────▼───────────────┐
             │          PostgreSQL           │
             └───────────────────────────────┘
```

Les deux façades partagent les mêmes handlers métier. `rmcp` expose son
transport HTTP sous forme de service Tower, monté dans le routeur Axum ; les
handlers d'outils accèdent aux en-têtes de la requête, donc au même contexte
d'authentification que l'API REST.

Le tout est déployable avec Docker Compose.

Éventuellement :

```text
Nginx
  ↓
frontend

/api/*  et  /mcp
  ↓
API Rust

PostgreSQL
```

Un stockage objet compatible S3 pourra être ajouté ultérieurement pour les
images intégrées aux questions.

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
Participant

Subject
SubjectVersion

Assessment
AssessmentParticipantOverride

Attempt
AttemptAnswer

GradingRevision

AuditEvent
```

Il n'existe pas d'entité `Student`. `Participant` ne porte aucun attribut
identifiant, et un participant appartient à un seul groupe à la fois — un
transfert est une mise à jour, ce qui rend inutile une table d'adhésion.

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
                           ├── Participant
                           └── AttemptAnswer
```

Les questions du sujet peuvent initialement être conservées sous forme de
document JSONB dans `SubjectVersion`.

Cela évite de transformer immédiatement chaque détail d'une question en une
multitude de tables relationnelles.

---

# 22. API indicative

L'API pourrait initialement proposer :

```text
POST   /api/auth/login                      enseignant
POST   /api/auth/token                      participant : jeton + secret

GET    /api/groups
POST   /api/groups
POST   /api/groups/{id}/participants        génère N jetons
GET    /api/groups/{id}/participants        jetons et états, jamais de noms
POST   /api/groups/{id}/purge

POST   /api/participants/{id}/reset-secret
POST   /api/participants/{id}/deactivate
POST   /api/participants/{id}/move

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

Les secrets initiaux ne sont retournés en clair que par la réponse à
`POST /api/groups/{id}/participants`, et ne sont plus jamais relisibles.

Aucun point d'entrée n'accepte de liste nominative. Il n'existe pas de
`POST /api/students/import`, et une telle route ne doit pas être ajoutée : la
production des billets est une opération strictement navigateur.

OpenAPI doit être généré directement depuis le backend Rust afin que le client
TypeScript puisse disposer de types générés.

La façade MCP décrite en section 9 est montée sur `/mcp` dans la même
application et s'appuie sur les mêmes handlers. Les outils qu'elle expose sont
délibérément taillés pour la tâche plutôt que calqués un à un sur les routes
REST : une transposition mécanique de l'OpenAPI produirait une surface
inutilisable par un agent.

---

# 23. MVP

Le MVP est volontairement limité.

Il doit permettre de réaliser l'intégralité du scénario suivant :

```text
 1. L'enseignant crée un groupe.

 2. Il demande N jetons pour ce groupe.

 3. Son navigateur joint ces jetons à sa liste locale
    et produit les billets imprimables.

 4. Il distribue les billets en classe.

 5. Un agent IA rédige un qcm/v1 YAML à partir du cours.

 6. L'agent le valide contre le schéma, corrige, revalide.

 7. L'agent dépose le sujet en brouillon via MCP
    et retourne l'URL de relecture.

 8. L'enseignant prévisualise le sujet dans son navigateur
    et le valide.

 9. L'agent crée une évaluation sur cette version validée
    et retourne le code.

10. L'enseignant ouvre l'évaluation.

11. Les élèves se connectent avec leur jeton.

12. Ils entrent le code.

13. Ils répondent individuellement au QCM.

14. Les réponses sont sauvegardées automatiquement.

15. Ils remettent leur copie.

16. Le backend calcule le résultat.

17. L'élève obtient le retour prévu par l'enseignant.

18. L'enseignant consulte les résultats par jeton
    et les statistiques par question.

19. Il charge sa liste locale pour afficher les noms.

20. Il exporte les résultats en CSV.
```

Côté interface web, le MVP se réduit donc au parcours élève complet et à quatre
écrans enseignant :

```text
prévisualisation et validation d'un sujet
génération de jetons et production des billets
ouverture / fermeture d'une évaluation
résultats, jointure locale et export
```

Tout le reste de l'administration passe par la façade MCP. Tout ce qui n'est pas
nécessaire à ce scénario doit être considéré avec prudence avant d'entrer dans
le MVP.

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
