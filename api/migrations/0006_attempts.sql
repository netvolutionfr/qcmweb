-- Sessions de participant.
--
-- La table des sessions sert les deux principaux ; `participant_id` NULL
-- désigne une session d'enseignant. Les extracteurs discriminent sur ce
-- champ : une session d'élève ne doit jamais ouvrir une route enseignante.
ALTER TABLE sessions
    ADD COLUMN participant_id uuid REFERENCES participants (id) ON DELETE CASCADE;

CREATE INDEX sessions_participant_idx ON sessions (participant_id);

-- Tentatives (SPEC §2 et §12).
CREATE TABLE attempts (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    assessment_id  uuid NOT NULL REFERENCES assessments (id) ON DELETE CASCADE,
    participant_id uuid NOT NULL REFERENCES participants (id) ON DELETE CASCADE,

    -- Graine du mélange. Conservée pour qu'une tentative reprise après une
    -- coupure retrouve exactement le même ordre de questions et de réponses.
    seed           bigint NOT NULL,

    -- Le serveur est la référence pour le temps (SPEC §12) : ces instants ne
    -- viennent jamais du navigateur.
    started_at     timestamptz NOT NULL DEFAULT now(),
    deadline       timestamptz,
    submitted_at   timestamptz,

    -- Résultat figé au moment de la correction.
    score          double precision,
    max_score      double precision,
    grade          double precision,
    breakdown      jsonb,

    CHECK (submitted_at IS NULL OR score IS NOT NULL)
);

CREATE INDEX attempts_assessment_idx ON attempts (assessment_id);
CREATE INDEX attempts_participant_idx ON attempts (assessment_id, participant_id);

CREATE TABLE attempt_answers (
    attempt_id  uuid NOT NULL REFERENCES attempts (id) ON DELETE CASCADE,
    question_id text NOT NULL,
    choices     jsonb NOT NULL DEFAULT '[]'::jsonb,
    updated_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (attempt_id, question_id)
);

-- Une copie remise ne se modifie plus (SPEC §12).
--
-- Garantie posée en base, comme l'immuabilité des versions : « remettre
-- définitivement » doit vouloir dire définitivement, y compris face à une
-- requête tardive qu'un contrôle applicatif aurait laissé passer.
CREATE FUNCTION answers_frozen_after_submit() RETURNS trigger AS $$
DECLARE
    submitted timestamptz;
    target uuid;
BEGIN
    target := coalesce(NEW.attempt_id, OLD.attempt_id);
    SELECT submitted_at INTO submitted FROM attempts WHERE id = target;

    IF submitted IS NOT NULL THEN
        RAISE EXCEPTION 'copie déjà remise : les réponses ne sont plus modifiables';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER attempt_answers_frozen
    BEFORE INSERT OR UPDATE OR DELETE ON attempt_answers
    FOR EACH ROW EXECUTE FUNCTION answers_frozen_after_submit();
