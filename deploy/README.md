# Déploiement

Cible : **Debian Trixie**, conteneurs Docker, **Nginx en frontal** assurant TLS.

```
Internet ──TLS──> Nginx (hôte)
                    │
                    ├── /api/, /mcp  ──> 127.0.0.1:3000  conteneur api
                    └── /            ──> 127.0.0.1:3001  front
                                          │
                                          └── postgres (réseau Docker)
```

Seul Nginx est exposé. L'API et Postgres ne publient leurs ports que sur la
boucle locale de l'hôte.

## Déploiement automatique

Un push sur `main` lance [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) :

```
tests ──> construction des images ──> déploiement ──> vérification publique
          (publiées sur GHCR)         (SSH, une commande)
```

Les images sont construites par la CI et **le serveur ne compile jamais** : il
récupère celles dont le tag est le SHA du commit, puis `docker compose up -d
--wait` attend que les healthchecks passent. Un déploiement qui échoue fait
échouer le workflow.

Les commits qui ne touchent que de la documentation (`*.md`, `docs/`) ne
déclenchent rien.

**Retour arrière** : dans l'onglet Actions, rouvrir un run antérieur réussi et
relancer son job `deploy`. Limite : les migrations ne vont que vers l'avant, un
retour au code précédent ne défait pas un changement de schéma.

## Mise en place, une seule fois

### 1. Sur le serveur

```bash
# Utilisateur dédié. Le groupe docker équivaut à root : c'est pourquoi la clé
# ci-dessous est restreinte à une seule commande.
sudo useradd --create-home --shell /bin/bash deploy
sudo usermod -aG docker deploy

sudo mkdir /opt/qcmweb && sudo chown deploy: /opt/qcmweb
sudo -u deploy git clone https://github.com/netvolutionfr/qcmweb.git /opt/qcmweb
```

Les secrets **applicatifs** vivent ici, sur le serveur, et nulle part dans
GitHub :

```bash
sudo -u deploy cp /opt/qcmweb/.env.example /opt/qcmweb/.env
sudo -u deploy chmod 600 /opt/qcmweb/.env
sudo -u deploy nano /opt/qcmweb/.env
```

Remplacer **toutes** les valeurs : `POSTGRES_PASSWORD` (aléatoire),
`TEACHER_PASSWORD_HASH` (voir plus bas, apostrophes simples obligatoires),
`TRUSTED_PROXY_CIDRS`. Le fichier n'est jamais versionné.

Empreinte du mot de passe enseignant, à générer sur votre poste :

```bash
cd api && cargo run -- hash-password
```

La saisie est masquée et demandée deux fois ; douze caractères au moins, une
phrase de passe de quelques mots convient très bien.

**Changer le mot de passe** : générer une nouvelle empreinte, la remplacer dans
le `.env` du serveur, puis redéployer ou redémarrer l'API. Toutes les sessions
ouvertes avec l'ancien sont alors refusées.

### 2. La clé de déploiement

Sur votre poste :

```bash
ssh-keygen -t ed25519 -N "" -C "github-actions qcmweb" -f qcmweb-deploy
```

Sur le serveur, dans `/home/deploy/.ssh/authorized_keys`, **sur une seule
ligne** :

```
command="/opt/qcmweb/deploy/deploy.sh",no-pty,no-port-forwarding,no-agent-forwarding,no-X11-forwarding ssh-ed25519 AAAA… github-actions qcmweb
```

Cette clé ne donne **aucun shell** : quoi qu'on lui demande, le serveur exécute
`deploy.sh`, qui ne sait déployer qu'un commit précis. Une clé volée ne permet
donc pas de prendre la machine.

Empreinte du serveur, à relever depuis un réseau de confiance et à comparer à
celle que donne `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub` sur le
serveur :

```bash
ssh-keyscan -t ed25519 mon-serveur.example
```

### 3. Dans GitHub

*Settings → Environments → New environment* : `production`, puis *Deployment
branches → Selected branches → `main`*. Ainsi un workflow poussé sur une autre
branche ne peut pas lire la clé.

Secrets **de l'environnement `production`** :

| Secret | Contenu |
|---|---|
| `DEPLOY_HOST` | nom d'hôte ou adresse du serveur |
| `DEPLOY_USER` | `deploy` |
| `DEPLOY_SSH_KEY` | contenu **complet** du fichier `qcmweb-deploy` (clé privée) |
| `DEPLOY_KNOWN_HOSTS` | la ligne rendue par `ssh-keyscan`, à l'identique |
| `DEPLOY_PORT` | *optionnel* — seulement si SSH n'écoute pas sur 22 |

Variable **de l'environnement** (non secrète) :

| Variable | Contenu |
|---|---|
| `PROD_URL` | `https://qcm.exemple.fr`, sans `/` final. Active la vérification publique après déploiement. |

Hors port 22, `ssh-keyscan -p <port>` produit une ligne `[hôte]:port …` : c'est
elle qu'il faut coller.

Ces secrets ne donnent accès qu'au déploiement. `DEPLOY_HOST` est un secret
parce que le dépôt est public, et que ses journaux le sont aussi.

Puis supprimer la clé privée de votre poste, elle n'a plus rien à y faire.

### 4. Nginx

Déposer [nginx.conf.example](nginx.conf.example) adapté dans
`/etc/nginx/sites-available/`, créer le lien dans `sites-enabled/`, obtenir le
certificat avec certbot, et `nginx -t && systemctl reload nginx`.

Le premier push sur `main` fait le reste.

### Mise à jour de `deploy.sh`

Le script exécuté est celui **déjà présent** sur le serveur, avant qu'il ne se
synchronise sur le nouveau commit. Une modification de `deploy.sh` ne prend donc
effet qu'au déploiement suivant.

## Démarrage manuel

Sans passer par la CI, pour un essai local ou un dépannage :

```bash
cp .env.example .env            # puis remplacer toutes les valeurs
docker compose up -d --build
```

## `TRUSTED_PROXY_CIDRS`, le réglage à ne pas manquer

L'API ne croit `X-Forwarded-For` que s'il provient d'un réseau déclaré ici.
Ce réglage n'a pas de valeur par défaut sûre, d'où son caractère obligatoire :

- **trop large** — n'importe qui peut annoncer l'adresse de son choix et
  contourner la limitation des tentatives de connexion ;
- **absent** — l'API s'en tient à l'adresse du pair, c'est-à-dire celle de
  Nginx pour tout le monde. La limitation par IP devient globale, et un tiers
  peut verrouiller l'enseignant hors de son instance.

Relever le réseau réel du conteneur :

```bash
docker network inspect qcmweb_default -f '{{range .IPAM.Config}}{{.Subnet}}{{end}}'
```

Nginx tournant sur l'hôte, il atteint le conteneur par la passerelle du pont
Docker. Le sous-réseau relevé ci-dessus est donc la valeur à déclarer, par
exemple `172.18.0.0/16`.

Vérification après démarrage — l'adresse journalisée doit être celle du client,
pas celle de la passerelle :

```bash
docker compose logs api | grep "connexion refusée"
```

## Brancher un agent sur la façade MCP

Créer une clé, qui ne sera affichée qu'une fois :

```bash
docker compose exec api qcmweb-api mint-key "Claude Code portable"
```

La déposer dans la configuration du client MCP — **jamais dans une
conversation**, un secret collé dans un prompt part chez un fournisseur de
modèle et se retrouve dans des journaux :

```json
{
  "mcpServers": {
    "qcmweb": {
      "type": "http",
      "url": "https://qcm.exemple.fr/mcp",
      "headers": { "Authorization": "Bearer qcmw_…" }
    }
  }
}
```

La clé porte le scope `agent` : elle permet de rédiger des sujets, de créer des
évaluations sur des versions validées et de lire des résultats pseudonymisés.
Elle ne permet ni de valider un sujet, ni d'ouvrir une évaluation, ni
d'approcher les participants.

Révoquer une clé :

```bash
docker compose exec -T postgres psql -U qcmweb -d qcmweb \
  -c "UPDATE api_keys SET revoked_at = now() WHERE label = 'Claude Code portable';"
```

## Sauvegarde

```bash
docker compose exec -T postgres pg_dump -U qcmweb qcmweb | gzip > qcmweb-$(date +%F).sql.gz
```

À conserver hors de la machine. Ce n'est pas facultatif : la base contient les
résultats de l'année, et [ADR-0001](../docs/adr/0001-pseudonymisation-par-jetons.md)
rappelle que la table de correspondance qui leur donne un sens vit, elle, sur le
poste de l'enseignant — les deux moitiés doivent survivre.
