# Debian Trixie de bout en bout : compiler sur une glibc plus récente que celle
# d'exécution produirait un binaire qui refuse de démarrer.
FROM rust:1.98-slim-trixie AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

# Les migrations sont embarquées par `sqlx::migrate!` à la compilation, d'où
# leur présence ici. Aucune base n'est nécessaire pour construire : le projet
# n'utilise pas les macros `query!` vérifiées à la compilation.
RUN cargo build --release --locked

# ponytail: pas de cache de dépendances (cargo-chef ou couche factice). Une
# reconstruction complète prend quelques minutes, ce qui reste acceptable pour
# un déploiement occasionnel. À ajouter si la CI devient fréquente.

FROM debian:trixie-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

# Utilisateur non privilégié : un processus applicatif n'a aucune raison
# d'être root, a fortiori celui qui est exposé au réseau.
RUN useradd --system --no-create-home --uid 10001 qcmweb
USER qcmweb

COPY --from=builder --chown=root:root /build/target/release/qcmweb-api /usr/local/bin/qcmweb-api

EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/qcmweb-api"]
