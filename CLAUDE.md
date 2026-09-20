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
6. **Aucune donnée d'élève transmise à un LLM.** L'IA intervient uniquement en
   amont, à la production du sujet, hors de l'application.

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
     │ HTTPS/JSON
   API        Rust · Axum · Tokio · Serde · SQLx · utoipa
     │
   Base       PostgreSQL (questions en JSONB dans SubjectVersion)
```

Déploiement par Docker Compose. Pas de stockage objet au MVP.

Arborescence cible :

```
api/    backend Rust
web/    frontend Next.js
docs/   schéma qcm/v1, notes d'architecture
```

---

## 4. Règles d'ingénierie

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

## 5. Périmètre

Le MVP est le scénario complet du §23 de la SPEC, adapté au modèle par jetons.
Avant d'ajouter quoi que ce soit, vérifier que c'est sur ce chemin.

Hors périmètre : classement, gamification, badges, chat, LMS, mobile natif,
notifications, marketplace, sandbox d'exécution de code, intégration Pronote,
analyse des réponses par IA.

Reporté après MVP : réponse numérique, réponse courte, regex, association,
classement, texte à trous, correction manuelle, OIDC/LDAP/ENT.

---

## 6. Commandes

```bash
docker compose up -d          # postgres + api + web
cargo test                    # api/   tests backend
cargo sqlx migrate run        # api/   migrations
npm run dev                   # web/   frontend
npm run typecheck             # web/
```

Les types TypeScript sont **générés** depuis l'OpenAPI produit par utoipa. Ne
jamais écrire à la main un type qui décrit une réponse d'API.

---

## 7. Conventions

- Interface et documentation utilisateur en **français**. Code, identifiants et
  messages de commit en **anglais**.
- Identifiants exposés à l'extérieur : UUID ou aléatoires, jamais séquentiels.
- Codes d'évaluation et jetons : alphabet sans caractères ambigus
  (`0/O`, `1/I/l`), entropie suffisante pour interdire l'énumération.
- Secrets hachés en **Argon2id**. Limitation du débit sur les endpoints
  d'authentification.
- Les opérations sensibles (modification de note, recalcul, purge) écrivent un
  `AuditEvent`.
- Toute logique non triviale de correction ou de barème laisse un test derrière
  elle.

---

## 8. Licence

MIT. Voir [LICENSE](LICENSE).
