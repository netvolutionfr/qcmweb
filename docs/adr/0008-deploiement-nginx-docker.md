# ADR-0008 — Déploiement Docker derrière Nginx, et frontière de confiance du proxy

**Statut** : Acceptée — 2026-09-20

## Contexte

L'hébergement retenu est un serveur **Debian Trixie**, l'application tournant en
conteneurs Docker, avec **Nginx en frontal** assurant la terminaison TLS.

Cette topologie a une conséquence qui n'est pas cosmétique : le backend ne voit
plus l'adresse de ses clients, mais celle du proxy.

## Décision

### Topologie

```
Internet ──TLS──> Nginx (hôte)
                    ├── /api/, /mcp ──> 127.0.0.1:3000  conteneur api
                    └── /           ──> 127.0.0.1:3001  front
                                          │
                                          └── postgres (réseau Docker)
```

Les conteneurs publient leurs ports **sur la seule boucle locale**. Seul Nginx
est exposé au réseau. L'API parle HTTP en clair à l'intérieur de l'hôte ; le
chiffrement s'arrête à Nginx.

### Même origine, donc pas de CORS

Le front et l'API sont servis sous la même origine, Nginx distinguant sur le
chemin. Aucun CORS, aucun préflight, et le cookie de session conserve
`SameSite=Strict`. En développement, un `rewrites` Next.js reproduit la même
disposition : les deux environnements se comportent identiquement.

### `X-Forwarded-For` n'est lu que depuis un réseau déclaré

C'est le cœur de cette ADR. `X-Forwarded-For` est un en-tête, donc trivial à
forger. Le lire inconditionnellement permettrait à n'importe qui d'annoncer une
adresse différente à chaque tentative et de contourner entièrement la
limitation des connexions de [ADR-0004](0004-mot-de-passe-enseignant.md).

L'API ne le lit donc que si **le pair appartient à un réseau déclaré dans
`TRUSTED_PROXY_CIDRS`**, et retient la **dernière** valeur de la chaîne : celle
que le proxy de confiance vient d'ajouter. Les valeurs précédentes peuvent
provenir du client.

Cette variable **n'a pas de valeur par défaut**. Les deux défauts imaginables
sont mauvais :

- *faire confiance à tout le monde* — faille silencieuse ;
- *ne faire confiance à personne sans le dire* — la limitation par IP devient
  globale, et un tiers peut verrouiller l'enseignant hors de son instance.

Elle est donc obligatoire côté Compose, et l'API journalise à son démarrage la
liste retenue, ou un avertissement explicite si elle est vide.

## Conséquences

- Un réglage erroné de `TRUSTED_PROXY_CIDRS` échoue **du côté sûr** : l'en-tête
  est ignoré et l'adresse du pair est utilisée. Observé en test — un CIDR ne
  correspondant pas au réseau réel a bien fait retomber l'API sur l'adresse de
  la passerelle. La vérification après déploiement est donc indispensable, et
  documentée dans [deploy/README.md](../../deploy/README.md).
- Les images sont construites et exécutées sur **Trixie de bout en bout** :
  compiler sur une glibc plus récente que celle d'exécution produirait un
  binaire refusant de démarrer.
- Le conteneur d'API tourne sous un **utilisateur non privilégié** (uid 10001).
- La sauvegarde de Postgres n'est pas facultative : la base détient les
  résultats, tandis que la table de correspondance qui leur donne un sens vit
  sur le poste de l'enseignant ([ADR-0001](0001-pseudonymisation-par-jetons.md)).
  Les deux moitiés doivent survivre séparément.

## Alternatives écartées

- **Traefik en frontal** — découverte automatique des conteneurs et TLS intégré,
  mais Nginx est déjà en place sur la machine et maîtrisé. Changer de frontal
  pour ce seul projet ajouterait une pièce à administrer.
- **TLS terminé par l'API** — il faudrait gérer les certificats dans le
  conteneur, et refaire ce que Nginx fait déjà pour les autres services de la
  machine.
- **CORS avec `Allow-Credentials`** — une configuration de sécurité à maintenir
  pour résoudre un problème qui disparaît avec trois lignes de proxy.
- **Publier les ports des conteneurs sur toutes les interfaces** — exposerait
  l'API sans TLS et Postgres au réseau.
