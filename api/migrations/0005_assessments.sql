-- Évaluations : l'usage d'une version précise d'un sujet dans des conditions
-- données (SPEC §2 et §10).

CREATE TABLE assessments (
    id                 uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_version_id uuid NOT NULL REFERENCES subject_versions (id),
    group_id           uuid NOT NULL REFERENCES groups (id) ON DELETE CASCADE,

    name               text NOT NULL,
    mode               text NOT NULL CHECK (mode IN ('FORMATIVE', 'SUMMATIVE')),

    -- Court, annoncé à voix haute ou projeté. Il n'authentifie rien : il
    -- désigne une évaluation ouverte, et l'appartenance au groupe reste
    -- vérifiée côté serveur (SPEC §11).
    code               text NOT NULL UNIQUE,

    state              text NOT NULL DEFAULT 'DRAFT' CHECK (state IN ('DRAFT', 'OPEN', 'CLOSED')),

    -- Fenêtre planifiée, indépendante de l'ouverture manuelle. Une évaluation
    -- n'est composable que si elle est OPEN *et* dans sa fenêtre : l'enseignant
    -- garde ainsi la main sans avoir à être devant son écran à l'heure dite.
    opens_at           timestamptz,
    closes_at          timestamptz,

    duration_minutes   integer CHECK (duration_minutes IS NULL OR duration_minutes > 0),
    max_attempts       integer NOT NULL DEFAULT 1 CHECK (max_attempts > 0),

    shuffle_questions  boolean NOT NULL DEFAULT true,
    shuffle_choices    boolean NOT NULL DEFAULT true,

    score_release      text NOT NULL DEFAULT 'AFTER_CLOSE'
                       CHECK (score_release IN ('IMMEDIATE', 'AFTER_CLOSE', 'NEVER')),
    correction_release text NOT NULL DEFAULT 'AFTER_CLOSE'
                       CHECK (correction_release IN ('IMMEDIATE', 'AFTER_CLOSE', 'NEVER')),

    max_grade          double precision NOT NULL DEFAULT 20 CHECK (max_grade > 0),

    created_at         timestamptz NOT NULL DEFAULT now(),
    opened_at          timestamptz,
    closed_at          timestamptz
);

CREATE INDEX assessments_group_idx ON assessments (group_id);
CREATE INDEX assessments_code_idx ON assessments (code) WHERE state = 'OPEN';

-- Un agent est autorisé à créer une évaluation (ADR-0003), et c'est
-- précisément cette contrainte qui rend ce pouvoir borné : il ne peut la
-- fonder que sur une version déjà relue et validée par l'enseignant.
--
-- Posée en base plutôt qu'en code, pour la même raison que l'immuabilité des
-- versions : la garantie ne doit pas dépendre du chemin emprunté.
CREATE FUNCTION assessment_needs_validated_version() RETURNS trigger AS $$
DECLARE
    version_status text;
BEGIN
    SELECT status INTO version_status
      FROM subject_versions WHERE id = NEW.subject_version_id;

    IF version_status IS DISTINCT FROM 'VALIDATED' THEN
        RAISE EXCEPTION
            'une évaluation ne peut porter que sur une version validée (statut : %)',
            coalesce(version_status, 'version inconnue');
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER assessments_validated_version
    BEFORE INSERT OR UPDATE OF subject_version_id ON assessments
    FOR EACH ROW EXECUTE FUNCTION assessment_needs_validated_version();

-- Aménagements individuels (SPEC §16), désignés par le jeton du participant.
-- Jamais de motif nominatif ni de mention relevant de la santé : l'enseignant
-- sait à qui correspond le jeton, le serveur non.
CREATE TABLE assessment_overrides (
    assessment_id  uuid NOT NULL REFERENCES assessments (id) ON DELETE CASCADE,
    participant_id uuid NOT NULL REFERENCES participants (id) ON DELETE CASCADE,
    extra_minutes  integer NOT NULL DEFAULT 0 CHECK (extra_minutes >= 0),
    extra_attempts integer NOT NULL DEFAULT 0 CHECK (extra_attempts >= 0),
    excused        boolean NOT NULL DEFAULT false,
    PRIMARY KEY (assessment_id, participant_id)
);
