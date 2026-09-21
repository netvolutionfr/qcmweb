-- Une session ne doit pas survivre au credential qui a servi à l'obtenir.
--
-- Avant ceci, la validation d'une session ne dépendait pas du secret courant :
-- une session volée restait valable jusqu'à trente jours après une
-- réinitialisation, précisément l'action qu'on fait pour couper un accès.

-- Enseignant : empreinte des identifiants en vigueur à l'ouverture de la
-- session. Les sessions ouvertes avant cette migration (empreinte NULL) sont
-- refusées : une reconnexion unique.
ALTER TABLE sessions ADD COLUMN credential text;

-- Participant : toute session est supprimée quand son secret change ou que le
-- participant est désactivé.
--
-- Posé en base plutôt qu'en code : la garantie doit tenir quel que soit le
-- chemin qui modifie la ligne — l'API, l'outil d'administration, ou une requête
-- tapée à la main un soir d'incident.
CREATE FUNCTION revoke_participant_sessions() RETURNS trigger AS $$
BEGIN
    DELETE FROM sessions WHERE participant_id = NEW.id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER participants_revoke_sessions
    AFTER UPDATE ON participants
    FOR EACH ROW
    WHEN (OLD.secret_hash IS DISTINCT FROM NEW.secret_hash
          OR OLD.active IS DISTINCT FROM NEW.active)
    EXECUTE FUNCTION revoke_participant_sessions();
