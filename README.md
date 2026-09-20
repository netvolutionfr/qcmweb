# QCMWeb

Plateforme légère de création, diffusion et correction d'évaluations par QCM,
pour l'enseignement secondaire et supérieur court.

L'objectif tient en une phrase : en moins de cinq minutes, passer d'un fichier
YAML à une évaluation accessible à une classe.

> **État du projet : embryonnaire.** La spécification fonctionnelle est écrite
> et le squelette du backend démarre. Il n'y a pas encore d'interface, pas
> d'authentification, et rien de tout cela n'est utilisable en classe.

---

## Deux partis pris

### Aucune donnée nominative hébergée

Le serveur ne stocke ni nom, ni prénom, ni adresse électronique, ni date de
naissance — et le schéma de base ne prévoit aucune colonne pour en accueillir.

Il connaît des **jetons** : `7K4M-P2QF` appartient au groupe `1SIO` et a obtenu
17/20. La correspondance entre un jeton et un élève réside uniquement sur le
poste de l'enseignant, dans un fichier qui ne quitte jamais sa machine. La
jointure qui affiche les vrais noms s'exécute dans son navigateur.

Conséquence directe : une compromission du serveur n'expose aucune identité, et
l'hébergeur ne traite aucune donnée nominative.

Ce n'est pas pour autant une sortie du RGPD — les jetons restent des données
pseudonymisées au sens du considérant 26, et le traitement doit figurer au
registre du responsable de traitement. Le détail de cette analyse, y compris ses
limites, est en [section 18 de la spécification](SPEC.md).

### L'administration passe par un agent, pas par des écrans

Les sujets étant le plus souvent rédigés par un agent IA, celui-ci dispose d'un
accès direct au backend via une **façade MCP**, montée dans la même application
que l'API REST et soumise aux mêmes autorisations.

L'agent rédige le QCM, le valide contre le schéma, se corrige, puis dépose le
sujet en brouillon et rend une URL de relecture. Cela évite d'avoir à coder une
interface d'administration complète.

La règle qui encadre ce pouvoir : **l'agent écrit, l'humain publie.**

| Agent (MCP) | Enseignant (navigateur) |
|---|---|
| valider un YAML | `DRAFT → VALIDATED` |
| créer un sujet en brouillon | ouvrir / fermer une évaluation |
| créer une évaluation | générer des jetons, produire les billets |
| lire des résultats pseudonymisés | jointure nominative et export |

Toute transition qui rend un contenu visible aux élèves, ou qui détruit des
données, reste manuelle. C'est aussi la défense contre l'injection de prompt :
un agent qui lirait un document hostile ne peut, au pire, que déposer un
brouillon indésirable.

---

## Le format `qcm/v1`

Format natif, éditable à la main, générable par un agent, validable par un
schéma. YAML pour l'écriture, JSON pour l'API et le stockage.

```yaml
schema: qcm/v1

metadata:
  title: "HTTP et architecture Web"
  tags: [http, web]

questions:
  - type: single_choice
    id: http-method-get
    prompt: |
      Quelle méthode HTTP est normalement utilisée
      pour demander une ressource sans la modifier ?
    choices:
      - { id: a, text: "POST" }
      - { id: b, text: "GET", correct: true }
      - { id: c, text: "DELETE" }
    points: 1
    explanation: "GET récupère la représentation d'une ressource."

  - type: multiple_choice
    id: http-codes
    prompt: "Lesquels correspondent à des erreurs côté client ?"
    choices:
      - { id: a, text: "200" }
      - { id: b, text: "404", correct: true }
      - { id: c, text: "403", correct: true }
      - { id: d, text: "500" }
    scoring: { mode: partial }
    points: 2
```

Les énoncés acceptent Markdown, coloration syntaxique comprise — indispensable
pour enseigner l'informatique.

Au-delà de la validation syntaxique, des règles métier sont appliquées :
`single_choice` exige exactement une réponse correcte, `multiple_choice` au
moins une, le barème doit être strictement positif, les identifiants doivent
être uniques. Le validateur renvoie **toutes** les erreurs d'un coup : un agent
qui corrige son fichier a besoin de la liste complète.

---

## Parcours

**Enseignant** — crée un groupe, génère N jetons, imprime les billets depuis son
navigateur, relit le sujet déposé par l'agent, le valide, ouvre l'évaluation,
consulte les résultats et les exporte.

**Élève** — connexion par jeton, saisie du code d'évaluation (`K7MP4Q`),
consignes, questionnaire, remise définitive, résultat selon les règles fixées
par l'enseignant.

Les réponses sont sauvegardées au fil de l'eau et une tentative interrompue peut
être reprise. La correction est intégralement serveur, et aucune bonne réponse
n'est transmise au navigateur avant que sa divulgation soit autorisée : ouvrir
les outils de développement ne révèle rien.

Pas de surveillance intrusive : ni détection de changement d'onglet, ni sortie
de plein écran, ni proctoring. Les garde-fous sont le mélange des questions et
des réponses, la durée et le nombre de tentatives.

---

## Stack

```
Navigateur   Next.js · TypeScript · types générés depuis OpenAPI
Agent IA     client MCP
     │ /api/*  et  /mcp
   API        Rust · Axum · Tokio · Serde · SQLx · utoipa · rmcp
     │
   Base       PostgreSQL
```

Une seule application Rust sert les deux façades : mêmes handlers, même
authentification, mêmes autorisations.

---

## Démarrage

```bash
cp .env.example .env            # puis remplacer les valeurs
docker compose up -d postgres
cd api && cargo run             # applique les migrations puis sert l'API
curl localhost:3000/api/health
```

Tests du backend : `cd api && cargo test`.

---

## Documentation

- [SPEC.md](SPEC.md) — spécification fonctionnelle complète, référence du projet.
- [CLAUDE.md](CLAUDE.md) — invariants et règles d'ingénierie, à lire avant de contribuer.

## Licence

[MIT](LICENSE).
