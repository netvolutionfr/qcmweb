#!/usr/bin/env bash
# Seul point d'entrée de la clé SSH de déploiement (ADR-0014).
#
# La clé est déclarée dans authorized_keys avec `command="…/deploy.sh"` : elle
# ne donne aucun shell, elle ne sait faire que ceci. L'utilisateur appartenant
# au groupe docker, autre chose reviendrait à donner root.
#
# Appel (par la CI) :  ssh deploy@hôte "<sha> <acteur>"  <<<"$GITHUB_TOKEN"
#   - <sha>    commit à déployer, 40 caractères hexadécimaux
#   - <acteur> compte GitHub, pour l'identification auprès de ghcr.io
#   - stdin    jeton éphémère du workflow, lecture seule sur les paquets
set -euo pipefail

APP_DIR="${APP_DIR:-/opt/qcmweb}"

die() { echo "deploy: $*" >&2; exit 2; }

# Tout le corps est dans une fonction : bash lit un script au fil de
# l'exécution, et `git reset` ci-dessous peut remplacer ce fichier. Une
# fonction est analysée en entier avant d'être exécutée.
main() {
  local sha actor extra
  read -r sha actor extra <<<"${SSH_ORIGINAL_COMMAND:-}"

  # La commande vient du réseau : on la valide avant de s'en servir.
  [[ "${sha:-}" =~ ^[0-9a-f]{40}$ ]]        || die "sha invalide"
  [[ "${actor:-}" =~ ^[A-Za-z0-9-]{1,39}$ ]] || die "acteur invalide"
  [[ -z "${extra:-}" ]]                      || die "arguments en trop"

  local token
  IFS= read -r token || die "jeton absent sur stdin"
  [[ -n "$token" ]] || die "jeton vide"

  cd "$APP_DIR"
  [[ -f .env ]] || die "$APP_DIR/.env introuvable : voir deploy/README.md"

  # Le jeton du workflow expire avec lui ; on ne laisse pas non plus
  # d'identifiants dans ~/.docker une fois le déploiement terminé.
  trap 'docker logout ghcr.io >/dev/null 2>&1 || true' EXIT
  docker login ghcr.io -u "$actor" --password-stdin <<<"$token" >/dev/null

  # Le dépôt est public : aucune authentification pour le récupérer. Se caler
  # sur le commit exact garde compose.yaml en phase avec les images déployées.
  git fetch --quiet --depth 1 origin "$sha"
  git reset --quiet --hard "$sha"

  export IMAGE_TAG="$sha"
  docker compose pull --quiet
  docker compose up -d --wait --wait-timeout 180 --remove-orphans

  # Anciennes images inutilisées : un déploiement par push, elles s'accumulent.
  docker image prune -af --filter "until=168h" >/dev/null
  echo "deploy: $sha en ligne"
}

main
exit
