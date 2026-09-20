-- Journal des opérations sensibles (SPEC §17) : purge, désactivation,
-- réinitialisation de secret, transfert, et plus tard modification de note et
-- recalcul de barème.
--
-- `details` ne contient jamais de donnée nominative : un journal d'audit est
-- typiquement ce qu'on conserve le plus longtemps, ce serait la pire table où
-- laisser fuir un nom.

CREATE TABLE audit_events (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    at         timestamptz NOT NULL DEFAULT now(),
    action     text NOT NULL,
    subject_id uuid,
    details    jsonb NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX audit_events_at_idx ON audit_events (at DESC);
CREATE INDEX audit_events_subject_idx ON audit_events (subject_id);
