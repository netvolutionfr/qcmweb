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

Les chemins ci-dessous (`/opt/qcmweb`, utilisateur `deploy`) sont des exemples :
n'importe quel dossier et n'importe quel nom d'utilisateur conviennent, y compris
le dossier personnel de l'utilisateur (par exemple `/home/qcm`). `deploy.sh` déduit
l'emplacement du dépôt de sa propre position ; il suffit que `authorized_keys`
désigne son chemin réel, et que `DEPLOY_USER` porte le nom de cet utilisateur.

```bash
# Utilisateur dédié. Le groupe docker équivaut à root : c'est pourquoi la clé
# ci-dessous est restreinte à une seule commande.
sudo useradd --create-home --shell /bin/bash deploy
sudo usermod -aG docker deploy

sudo mkdir /opt/qcmweb && sudo chown deploy: /opt/qcmweb
sudo -u deploy git clone https://github.com/netvolutionfr/qcmweb.git /opt/qcmweb
```

Le premier clonage est **forcément manuel** : `deploy.sh` fait partie du dépôt, il
n'existe pas sur le serveur tant que le dépôt n'y est pas. Les déploiements
suivants, eux, sont faits par le script.

`git clone` refuse un dossier non vide, ce qui est le cas d'un dossier personnel
(il contient au moins `.ssh`). Dans ce cas, initialiser le dépôt sur place :

```bash
cd /home/qcm
git init -b main
git remote add origin https://github.com/netvolutionfr/qcmweb.git
git fetch --depth 1 origin main
git reset --hard FETCH_HEAD
ls -l deploy/deploy.sh          # doit afficher -rwxr-xr-x
```

Un sous-dossier dédié (`/home/qcm/app`, où `git clone` fonctionne directement)
garde le dossier personnel net ; `authorized_keys` doit alors désigner
`/home/qcm/app/deploy/deploy.sh`.

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

Empreinte du serveur (`DEPLOY_KNOWN_HOSTS`). Elle permet au workflow de vérifier
qu'il parle bien à **votre** serveur et pas à un intercepteur. Ce n'est pas la
commande qu'on colle dans le secret, mais **ce qu'elle affiche** :

```bash
ssh-keyscan mon-serveur.example 2>/dev/null | grep -v '^#'
```

`mon-serveur.example` est le nom (ou l'adresse) du serveur, **exactement** tel que
vous le mettrez dans `DEPLOY_HOST`. Le `grep` retire les lignes de commentaire,
que `ssh-keyscan` écrit sur la sortie standard. La commande affiche une ligne par
type de clé que le serveur propose, par exemple :

```
mon-serveur.example ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMq3k8f9x2Qw7Hn1B0vZp…
mon-serveur.example ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQDNIhoG+yGoSDkmIfkp4ju…
```

Un serveur n'en propose parfois qu'un seul (RSA, par exemple), ce qui suffit.
Ces lignes, **entières**, vont dans le secret. Elles sont publiques : rien de
confidentiel, mais elles doivent être exactes.

`ssh-keyscan` fait confiance à qui répond. Pour être sûr de ne pas avoir pris
l'empreinte d'un intercepteur, comparez ces deux résultats, dont les empreintes
`SHA256:…` doivent être identiques :

```bash
# sur votre poste
ssh-keyscan mon-serveur.example 2>/dev/null | grep -v '^#' | ssh-keygen -lf -
# sur le serveur
ssh-keygen -lf /etc/ssh/ssh_host_*_key.pub
```

Si elles diffèrent, **ne collez rien** : vous ne parlez pas à la machine que vous
croyez, ou quelqu'un s'interpose.

Si SSH n'écoute pas sur le port 22, ajouter `-p <port>` à `ssh-keyscan` : la ligne
commence alors par `[mon-serveur.example]:<port>`.

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
| `DEPLOY_KNOWN_HOSTS` | la ligne **affichée** par `ssh-keyscan` (voir ci-dessus), pas la commande |
| `DEPLOY_PORT` | *optionnel* — seulement si SSH n'écoute pas sur 22 |

Variable **de l'environnement** (non secrète) :

| Variable | Contenu |
|---|---|
| `PROD_URL` | `https://qcm.exemple.fr`, sans `/` final. Active la vérification publique après déploiement. |

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

**Au tout premier démarrage**, ce réseau n'existe pas encore, et Compose refuse de
démarrer si la variable est vide. Commencer par `172.16.0.0/12`, la plage où
Docker crée ses réseaux privés. C'est une valeur raisonnable à garder : le port de
l'API n'est publié que sur la boucle locale, donc seuls des processus de la
machine peuvent l'atteindre.

Une valeur plus étroite (le sous-réseau relevé ci-dessus) est plus stricte, mais
fragile : `docker compose down` supprime le réseau, et le `up` suivant peut lui
donner un autre sous-réseau. L'erreur ne se voit alors pas au démarrage, mais dans
le journal : toutes les connexions y portent l'adresse de la passerelle.

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
