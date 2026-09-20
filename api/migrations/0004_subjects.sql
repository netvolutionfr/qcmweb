-- Banque de sujets et versions immuables (SPEC §5).
--
-- Répartition des états : `DRAFT`/`VALIDATED` portent sur une **version**,
-- puisque c'est une version que l'on relit puis que l'on valide, et qu'un même
-- sujet peut avoir une v1 validée et une v2 en brouillon. `ARCHIVED` porte sur
-- le **sujet**, car il retire l'ensemble de la banque courante sans rien dire
-- des versions déjà utilisées par des évaluations passées.

CREATE TABLE subjects (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    created_at  timestamptz NOT NULL DEFAULT now(),
    archived_at timestamptz
);

CREATE TABLE subject_versions (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_id   uuid NOT NULL REFERENCES subjects (id) ON DELETE CASCADE,
    number       integer NOT NULL,
    status       text NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'VALIDATED')),

    -- Le document qcm/v1 complet. Conserver la question sous forme de document
    -- plutôt qu'éclatée en tables évite de figer prématurément un modèle
    -- relationnel pour des types de questions encore à venir (SPEC §21).
    document     jsonb NOT NULL,

    -- Dénormalisés pour lister sans désérialiser chaque document.
    title        text NOT NULL,
    question_count integer NOT NULL,
    total_points double precision NOT NULL,

    created_at   timestamptz NOT NULL DEFAULT now(),
    validated_at timestamptz,

    UNIQUE (subject_id, number)
);

CREATE INDEX subject_versions_subject_idx ON subject_versions (subject_id, number DESC);

-- Immuabilité (SPEC §5) : une évaluation référence une version précise, et
-- modifier aujourd'hui un sujet utilisé il y a trois mois ne doit jamais
-- altérer rétroactivement l'évaluation passée.
--
-- La garantie est posée en base plutôt que confiée à la discipline du code :
-- c'est une exigence d'auditabilité, et une requête d'administration lancée à
-- la main doit s'y heurter comme le reste.
CREATE FUNCTION subject_version_is_immutable() RETURNS trigger AS $$
BEGIN
    IF NEW.document IS DISTINCT FROM OLD.document THEN
        RAISE EXCEPTION
            'une version de sujet est immuable : créer une nouvelle version plutôt que modifier la %', OLD.number;
    END IF;
    IF OLD.status = 'VALIDATED' AND NEW.status = 'DRAFT' THEN
        RAISE EXCEPTION 'une version validée ne peut pas redevenir un brouillon';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER subject_versions_immutable
    BEFORE UPDATE ON subject_versions
    FOR EACH ROW EXECUTE FUNCTION subject_version_is_immutable();
