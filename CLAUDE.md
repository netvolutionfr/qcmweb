# QCMWeb

Plateforme d'évaluation par QCM pour l'enseignement secondaire et supérieur court
(BTS SIO, Terminale NSI). La spécification fonctionnelle de référence est
[SPEC.md](SPEC.md) — la lire avant toute décision d'architecture.

Règle produit : *en moins de cinq minutes, un enseignant disposant d'un YAML
`qcm/v1` doit pouvoir le transformer en évaluation accessible à ses élèves.*

---

## 1. Invariants RGPD — non négociables

Ces règles priment sur toute autre considération, y compris le confort d'usage.
Toute PR qui en enfreint une est à rejeter.

1. **Aucune donnée nominative sur le serveur.** Pas de `nom`, `prenom`, `email`,
   `date_naissance`, ni aucun champ libre susceptible d'en accueillir un.
   Il n'existe pas de table `Student`. L'entité serveur est `Participant`, et
   elle ne porte qu'un jeton, un groupe et un état.
2. **Aucun endpoint d'upload de liste d'élèves.** L'import CSV nominatif est
   une opération *exclusivement navigateur* (File API). Le CSV n'est jamais
   transmis, ni en clair, ni chiffré, ni en pièce jointe.
3. **La jointure jeton ↔ identité est locale.** Elle se fait en mémoire dans le
   navigateur de l'enseignant, ou sur papier. Le résultat d'une jointure ne
   remonte jamais au serveur.
4. **Rétention explicite.** Toute donnée rattachable à une personne (jetons,
   tentatives, résultats) porte une date d'expiration et une purge effective.
   Pas de conservation « au cas où ».
5. **Aucun tracker, aucune analytics tierce, aucune police distante.**
   Zéro requête sortante depuis le navigateur vers un domaine tiers.
6. **Aucune donnée d'élève transmise à un LLM.** Tout ce qui transite par un
   outil MCP entre dans le contexte d'un modèle : aucun outil ne touche donc aux
   participants, aux jetons, ni à la table de correspondance locale. L'IA rédige
   des sujets et lit des résultats pseudonymisés, rien d'autre.

**Modèle mental** : le serveur est un paquet de copies *sans nom dessus*. Il
calcule des scores pour des jetons. Seul l'enseignant, sur son poste, sait à
qui correspond `7K4M-P2QF`.

Si une fonctionnalité demandée semble exiger un nom côté serveur, c'est que la
fonctionnalité doit être repensée — ou réalisée côté client. Le signaler plutôt
que contourner.

---

## 2. Modèle d'identité

Décisions arrêtées (2026-09-20) :

- **Jetons stables à l'année.** Un participant conserve son jeton sur toutes les
  évaluations de l'année scolaire. Conséquence assumée : le serveur détient un
  répertoire pseudonyme durable, le RGPD s'y applique, une ligne au registre de
  l'établissement est nécessaire. La purge de fin d'année est donc un vrai
  chemin de code, pas une intention.
- **Billets générés localement.** L'enseignant crée un groupe, demande N jetons,
  et son navigateur joint ces jetons à son CSV local pour produire les billets
  imprimables (nom + jeton + secret). Le serveur ne voit que les jetons.
- **Hébergement** : VPS personnel, UE. L'hébergeur est sous-traitant au sens de
  l'art. 28 ; la surface reste minimale puisqu'il n'y a aucun nom à héberger.
- **Usage strictement personnel.** Ouvrir l'instance à des collègues change le
  statut de responsable de traitement. Ne pas construire de multi-tenant sans
  remettre cette analyse à plat.

Authentification élève : `jeton + secret` (Argon2id côté serveur), le code
d'évaluation à 6 caractères jouant le rôle de second facteur contextuel.

---

## 3. Stack

```
Navigateur   Next.js (App Router) · TypeScript · types générés depuis OpenAPI
Agent IA     client MCP
     │ /api/*  et  /mcp
   API        Rust · Axum · Tokio · Serde · SQLx · utoipa · rmcp
     │
   Base       PostgreSQL (questions en JSONB dans SubjectVersion)
```

La façade MCP (`rmcp`, `StreamableHttpService` monté comme service Tower dans le
routeur Axum) n'est **pas un second service** : même binaire, même middleware
d'authentification, mêmes handlers métier. Elle n'ouvre aucun chemin d'accès qui
n'existe pas déjà en REST.

Déploiement par Docker Compose. Pas de stockage objet au MVP.

Arborescence cible :

```
api/    backend Rust
web/    frontend Next.js
docs/   schéma qcm/v1, notes d'architecture
```

---

## 4. Partage agent / humain

L'administration passe par la façade MCP plutôt que par des écrans. Le front
enseignant se réduit à quatre écrans : prévisualisation et validation d'un
sujet, génération de jetons et billets, ouverture/fermeture d'une évaluation,
résultats avec jointure locale et export.

**Règle : l'agent écrit, l'humain publie.** Toute transition qui rend quelque
chose visible aux élèves, ou qui détruit des données, relève exclusivement du
navigateur.

| Agent (MCP) | Enseignant (navigateur) |
|---|---|
| valider un YAML (dry-run) | `DRAFT → VALIDATED` |
| créer un sujet en `DRAFT` | ouvrir / fermer une évaluation |
| lire sujets et versions | générer des jetons, réinitialiser un secret |
| créer une évaluation (version validée requise) | purger un groupe |
| lire résultats pseudonymisés et stats | jointure nominative, billets, export nominatif |

Cette règle est appliquée par un **scope `agent`** porté par le jeton d'API et
vérifié dans le middleware — pas par convention, pas par la description des
outils.

Elle est aussi la défense contre l'injection de prompt : un agent qui lit un
document hostile ne peut, au pire, que déposer un brouillon indésirable. La
porte `DRAFT → VALIDATED` humaine est la protection réelle. Ne jamais
l'automatiser, même « pour les évaluations formatives ».

---

## 5. Authentification

Deux principaux, deux mécanismes, jamais interchangeables :

| | Enseignant | Agent |
|---|---|---|
| preuve | mot de passe Argon2id | clé d'API |
| transport | cookie de session `HttpOnly` | `Authorization: Bearer` |
| stockage | empreinte Argon2id (config) | empreinte SHA-256 (base) |
| extracteur | `Teacher` | `Agent` |

Règles :

- **Argon2id pour les mots de passe, SHA-256 pour les clés.** Un KDF ne sert
  qu'à ralentir la recherche exhaustive d'un secret *devinable* ; sur 256 bits
  aléatoires il ne ferait que coûter 100 ms par requête d'agent.
- **La base ne contient jamais un secret en clair**, ni mot de passe, ni jeton
  de session, ni clé. Uniquement des empreintes.
- **Un handler protégé prend `Teacher` ou `Agent` en argument.** L'absence du
  type est alors une erreur de compilation, pas un middleware oublié.
- **Pas d'inscription, pas de réinitialisation en libre-service.** L'instance
  sert un seul enseignant. Un courriel de récupération supposerait exactement le
  type de donnée que l'application refuse de détenir.
- **Passkeys reportées**, pas abandonnées : elles remplaceront le mot de passe.
  Ne pas empiler d'astuces autour du mot de passe en attendant.
- **Les clés se créent en ligne de commande** (`cargo run -- mint-key <libellé>`),
  s'affichent une fois, et vivent dans la configuration du client MCP — jamais
  dans une conversation.

Outils MCP et ressource : voir SPEC.md §9. Les outils sont taillés pour la
tâche, pas transposés un à un depuis les routes REST — un pont OpenAPI→MCP
générique produirait une surface inutilisable par un agent.

---

## 6. Règles d'ingénierie

- **Le backend est l'unique source de vérité.** Heure de début, durée, heure de
  remise et calcul du score sont serveur. Le client ne fait qu'afficher.
- **Aucune réponse correcte envoyée prématurément.** Les champs `correct`,
  `explanation` et le barème détaillé sont retirés du payload tant que leur
  divulgation n'est pas autorisée par la configuration de l'évaluation. C'est un
  filtrage à la sérialisation, pas une affaire de composant React.
- **Immuabilité des versions de sujet.** Une évaluation référence une
  `SubjectVersion` figée. Modifier un sujet crée une version, jamais une
  mutation en place.
- **Correction reproductible et auditable.** Un recalcul (neutralisation d'une
  question, barème corrigé) crée une `GradingRevision` horodatée et motivée. Les
  résultats antérieurs restent reconstituables.
- **Le format interne `qcm/v1` ne dépend d'aucun format externe.** AMC-TXT,
  Moodle XML, GIFT sont des adaptateurs en entrée. Aucune de leurs notions ne
  remonte dans le cœur.
- **Types de questions modélisés par un enum Rust**, pas par un champ `kind:
  String` accompagné de champs optionnels. Les états incohérents doivent être
  inexprimables.
- **Validation métier explicite** : `single_choice` → exactement une réponse
  correcte ; `multiple_choice` → au moins une ; `points > 0` ; unicité des ID de
  question et des ID de choix dans une question.
- **Markdown** dans les énoncés et les propositions, avec coloration syntaxique
  du code. Assainir le rendu (pas de HTML brut injecté).
- **Pas de proctoring.** Ni détection de changement d'onglet, ni sortie de
  plein écran, ni capture. La SPEC l'exclut explicitement (§17).

---

## 7. Périmètre

Le MVP est le scénario complet du §23 de la SPEC (modèle par jetons,
administration par agent).
Avant d'ajouter quoi que ce soit, vérifier que c'est sur ce chemin.

Hors périmètre : classement, gamification, badges, chat, LMS, mobile natif,
notifications, marketplace, sandbox d'exécution de code, intégration Pronote,
analyse des réponses par IA.

Reporté après MVP : réponse numérique, réponse courte, regex, association,
classement, texte à trous, correction manuelle, OIDC/LDAP/ENT.

---

## 8. Commandes

```bash
cp .env.example .env            # une fois, puis remplacer les valeurs
docker compose up -d postgres   # Postgres, port hôte 5434 par défaut
cd api && cargo run             # applique les migrations puis sert l'API
cd api && cargo test            # tests backend
curl localhost:3000/api/health  # vérifie API + base
```

Un **seul `.env`, à la racine**, sert à la fois à l'interpolation Compose et à
l'API : `dotenvy` remonte l'arborescence, on peut donc lancer depuis `api/`
comme depuis la racine. Deux fichiers `.env` finiraient par diverger sur le mot
de passe.

Pile complète en conteneurs (ce que fait la production) :

```bash
docker compose up -d --build     # postgres + api
docker compose logs -f api
```

Déploiement : Debian Trixie, Docker, Nginx en frontal pour TLS. Voir
[deploy/README.md](deploy/README.md) — et en particulier `TRUSTED_PROXY_CIDRS`,
qui n'a pas de valeur par défaut sûre.

Les migrations sont appliquées au démarrage par `sqlx::migrate!`, il n'y a donc
pas d'étape manuelle. Le port hôte par défaut est 5434 parce que 5432 et 5433
sont déjà occupés par d'autres projets sur le poste de développement ;
`POSTGRES_PORT` permet d'en changer.

Front :

```bash
cd web && npm run dev        # port 3001, proxifie /api vers le backend
cd web && npm run types      # régénère lib/openapi.d.ts depuis l'OpenAPI
cd web && npm test           # node --test sur la logique pure
cd web && npm run typecheck
```

`npm run types` exige que le backend tourne. À relancer **dès qu'une route ou un
schéma change** : un type écrit à la main est une régression, pas un raccourci.

Arborescence du backend :

```
api/
  migrations/         SQL, appliqué au démarrage
  src/
    main.rs           amorçage : tracing, config, pool, migrations, serve
    config.rs         lecture de l'environnement, échec au démarrage
    state.rs          AppState partagé
    error.rs          AppError -> réponse HTTP, jamais de détail SQL au client
    routes/           façade REST
    domain/qcm.rs     modèle natif qcm/v1 et validations métier
```

Arborescence du front :

```
web/
  app/
    layout.tsx           polices, thème, métadonnées
    connexion/           public
    (app)/               écrans authentifiés
      layout.tsx         garde de session + en-tête
      billets/
  components/
    ui/                  shadcn, copié dans le dépôt donc modifiable
    billets/
  lib/
    api.ts               client typé, même origine
    openapi.d.ts         généré, ne jamais éditer
    roster.ts            lecture CSV et jointure — strictement locale
```

Règles du front :

- **`lib/roster.ts` ne doit jamais appeler `fetch`.** C'est le module qui
  manipule les noms ; l'invariant RGPD tient à ce qu'il reste hors réseau.
- **Thème par `prefers-color-scheme`**, pas par classe : aucune dépendance,
  aucun flash avant hydratation. Une bascule manuelle exigerait `next-themes`
  et le retour au variant par classe (voir ADR-0009).
- **Mobile d'abord.** Le parcours élève se fera en salle, sur téléphone.
- La garde de session côté client n'est pas un contrôle de sécurité :
  l'autorisation est vérifiée par le serveur à chaque requête.

Les types TypeScript sont **générés** depuis l'OpenAPI produit par utoipa. Ne
jamais écrire à la main un type qui décrit une réponse d'API.

---

## 9. Conventions

- Interface et documentation utilisateur en **français**. Code, identifiants et
  messages de commit en **anglais**.
- Identifiants exposés à l'extérieur : UUID ou aléatoires, jamais séquentiels.
- Codes d'évaluation et jetons : alphabet sans caractères ambigus
  (`0/O`, `1/I/l`), entropie suffisante pour interdire l'énumération.
- Secrets **jamais stockés en clair** ; voir la section Authentification pour le
  choix de l'empreinte selon l'entropie du secret.
- Les opérations sensibles (modification de note, recalcul, purge) écrivent un
  `AuditEvent`.
- **Aucun secret dans un fichier versionné**, y compris un mot de passe de
  développement sans valeur : c'est l'habitude qui protège, pas la sensibilité
  du secret concerné. Les identifiants vivent dans `.env`, ignoré par git ;
  `.env.example` ne contient que des valeurs de remplacement explicites.
- `compose.yaml` (formalisme Compose V2, sans clé `version:`) n'écrit jamais un
  identifiant en dur. Il interpole `${VAR:?message}` : une variable manquante
  fait échouer la commande avec une consigne utile, plutôt que de démarrer sur
  une valeur par défaut silencieuse.
- Toute logique non triviale de correction ou de barème laisse un test derrière
  elle.
- **Toute décision d'architecture donne lieu à une ADR** dans `docs/adr/`, dans
  la foulée de la décision et non « plus tard ». Une ADR se justifie dès qu'un
  choix pourrait raisonnablement être contesté, repose sur une contrainte non
  évidente, ou assume un compromis. Elle dit surtout **pourquoi pas la solution
  évidente** : c'est cette partie qui manque toujours six mois après.
  Une ADR acceptée est immuable — une décision qui change en produit une
  nouvelle, l'ancienne passant en *Remplacée par*. Voir
  [docs/adr/README.md](docs/adr/README.md).

---

## 10. Historique des décisions

Les choix structurants sont consignés dans [docs/adr/](docs/adr/). À lire avant
de remettre en cause une contrainte qui paraît arbitraire — elle a
probablement une raison, et cette raison y est écrite.

---

## 11. Licence

MIT. Voir [LICENSE](LICENSE).
