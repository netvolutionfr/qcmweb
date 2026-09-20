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

## Mise en route

```bash
cp .env.example .env            # puis remplacer toutes les valeurs
cd api && cargo run -- hash-password   # empreinte du mot de passe enseignant
docker compose up -d --build
```

Puis déposer [nginx.conf.example](nginx.conf.example) adapté dans
`/etc/nginx/sites-available/`, créer le lien dans `sites-enabled/`, obtenir le
certificat avec certbot, et `nginx -t && systemctl reload nginx`.

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
