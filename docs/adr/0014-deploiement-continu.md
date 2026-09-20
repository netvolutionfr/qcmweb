# ADR-0014 — Déploiement continu : images construites en CI, clé SSH à commande forcée

**Statut** : Acceptée — 2026-09-20

## Contexte

Un push sur `main` doit mettre à jour la production, sans staging ni
environnement de développement : le projet n'a pas d'enjeu stratégique. La cible
est un serveur Debian Trixie unique, sous Docker, derrière Nginx
([ADR-0008](0008-deploiement-nginx-docker.md)). Le dépôt est public, donc ses
journaux d'Actions aussi.

## Décision

### Les images sont construites par la CI, jamais par le serveur

La CI teste, construit les images `api` et `web`, les publie sur GHCR sous le
SHA du commit, puis demande au serveur de les récupérer.

Compiler le binaire Rust sur un petit VPS coûte plusieurs minutes de CPU et une
mémoire que la production n'a pas à réserver. Et l'image déployée est celle qui
a suivi les tests, pas une reconstruction : ce qui tourne est ce qui a été
vérifié.

### Les secrets applicatifs restent sur le serveur

Le mot de passe de la base et l'empreinte du mot de passe enseignant vivent dans
un `.env` sur le serveur. GitHub ne détient que **l'accès de déploiement**
(hôte, utilisateur, clé, empreinte du serveur).

Une compromission de GitHub donne alors la capacité de déclencher un
déploiement, pas les données d'authentification de l'application.

### La clé SSH ne donne aucun shell

Elle est déclarée dans `authorized_keys` avec `command="…/deploy.sh"`. Quoi
qu'on lui demande, le serveur exécute ce script, qui accepte un SHA et un nom
d'acteur, tous deux validés par expression régulière avant tout usage.

Ce durcissement est proportionné à un risque précis : l'utilisateur de
déploiement appartient au groupe `docker`, ce qui équivaut à root. Une clé
ordinaire volée serait une prise de contrôle de la machine ; celle-ci ne permet
que de redéployer un commit du dépôt public.

### Le jeton GHCR est celui du workflow, pas un jeton personnel

Le `GITHUB_TOKEN`, lisible sur les paquets et valable le temps du job, est
transmis au script sur l'entrée standard. Aucun jeton d'accès personnel de longue
durée n'est stocké nulle part, et le script se déconnecte du registre à la sortie.

### Les actions sont épinglées par SHA

Une étiquette de version d'action est déplaçable par son propriétaire : la
référencer par étiquette, c'est exécuter demain du code que l'on n'a pas relu
aujourd'hui, avec accès aux secrets. Dependabot maintient les épinglages, sans
quoi ils vieilliraient en silence.

L'empreinte du serveur est de même **fournie** au workflow, pas obtenue par
`ssh-keyscan` : un `ssh-keyscan` lancé pendant le déploiement accepterait
n'importe quel serveur et ne vérifierait plus rien.

### Un environnement `production` limité à `main`

Les secrets sont rattachés à cet environnement, accessible seulement aux jobs
exécutés depuis `main`. Un workflow ajouté sur une autre branche ne peut pas lire
la clé.

## Conséquences

- Aucune vérification n'est faite par le serveur au-delà de `--wait` : un
  déploiement dont les healthchecks échouent fait échouer le job, mais l'ancienne
  version a déjà été remplacée. Pas de bascule progressive ni de retour arrière
  automatique — choix assumé pour un projet sans enjeu.
- Le retour arrière consiste à relancer le job `deploy` d'un run antérieur. Les
  migrations ne vont que vers l'avant : revenir au code précédent ne défait pas
  un changement de schéma incompatible.
- Une brève interruption a lieu à chaque déploiement, le temps du remplacement
  des conteneurs.
- `deploy.sh` s'exécute dans sa version **déjà présente** sur le serveur, avant
  synchronisation sur le nouveau commit : une modification du script ne prend
  effet qu'au déploiement suivant.
- Le corps du script est enveloppé dans une fonction : bash lit un script au fil
  de l'exécution, et le `git reset` qu'il lance peut le remplacer sous ses pieds.

## Alternatives écartées

- **Compiler sur le serveur** — lent, gourmand, et ce qui tourne n'est plus ce
  qui a été testé.
- **Watchtower ou toute mise à jour par tirage automatique** — déploie sans
  attendre les tests, et exige l'accès au socket Docker depuis un conteneur.
- **Secrets applicatifs dans GitHub, écrits sur le serveur à chaque déploiement**
  — élargit ce qu'un accès à GitHub permet d'obtenir, pour éviter de gérer un
  fichier une fois.
- **Une action tierce de SSH** — un code de plus, qui manipule la clé privée, pour
  faire ce qu'`ssh` fait en deux lignes.
- **Jeton d'accès personnel pour tirer les images** — un secret de longue durée à
  faire tourner, là où le jeton du workflow suffit.
- **Rendre les images publiques** — évite le jeton, mais dépend d'un réglage
  manuel dans l'interface de GHCR que rien ne vérifie.
