-- Deux principaux, deux mécanismes.
--
-- L'enseignant s'authentifie par mot de passe (Argon2id, hash en configuration)
-- et obtient une session. L'agent s'authentifie par clé d'API : jamais tapée
-- par un humain, donc pleine entropie, donc SHA-256 suffit — Argon2 ne sert
-- qu'à compenser la faiblesse d'un secret choisi par une personne.
--
-- Dans les deux cas la base ne stocke qu'une empreinte : une lecture de la base
-- ne doit pas suffire à usurper une session ni une clé.

CREATE TABLE sessions (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    token_hash text NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL
);

CREATE INDEX sessions_expiry_idx ON sessions (expires_at);

CREATE TABLE api_keys (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    label        text NOT NULL,             -- "Claude Code portable", pour savoir quoi révoquer
    key_hash     text NOT NULL UNIQUE,
    scope        text NOT NULL,             -- 'agent' pour l'instant
    created_at   timestamptz NOT NULL DEFAULT now(),
    last_used_at timestamptz,
    revoked_at   timestamptz
);

CREATE INDEX api_keys_active_idx ON api_keys (key_hash) WHERE revoked_at IS NULL;
