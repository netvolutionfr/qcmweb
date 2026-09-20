-- Invariant : aucune colonne nominative, ici ou ailleurs.
-- La correspondance jeton -> nom, prénom réside uniquement sur le poste de
-- l'enseignant. Voir SPEC.md §18 et CLAUDE.md §1.

CREATE TABLE groups (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    label       text NOT NULL,              -- "1SIO", "2SIO-SLAM"... jamais un nom de personne
    school_year text NOT NULL,              -- "2026-2027"
    created_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (label, school_year)
);

CREATE TABLE participants (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id    uuid NOT NULL REFERENCES groups (id) ON DELETE CASCADE,
    token       text NOT NULL UNIQUE,       -- tiré aléatoirement, dérivé d'aucune donnée personnelle
    secret_hash text NOT NULL,              -- Argon2id
    active      boolean NOT NULL DEFAULT true,
    expires_at  timestamptz NOT NULL,       -- purge de fin d'année
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX participants_group_idx ON participants (group_id);
CREATE INDEX participants_expiry_idx ON participants (expires_at);
