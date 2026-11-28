-- What each participant reports about itself. Changes bump `status_revision` instead of the
-- room's revision, so replicas forward them without reloading the room.
ALTER TABLE watch_room_participants
    ADD COLUMN sync TEXT CHECK (sync IN ('synced', 'buffering', 'behind')),
    ADD COLUMN place TEXT CHECK (place IN ('watching', 'browsing', 'shorts')),
    ADD COLUMN status_revision BIGINT NOT NULL DEFAULT 0;

-- Payload: room id, participant id, status revision, sync and place (`-` for none).
CREATE FUNCTION watch_room_status_notify() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_notify('watch_room_status', concat_ws(' ', NEW.room_id, NEW.id,
        NEW.status_revision, COALESCE(NEW.sync, '-'), COALESCE(NEW.place, '-')));
    RETURN NULL;
END
$$;

CREATE TRIGGER watch_room_status_notify
    AFTER UPDATE OF status_revision ON watch_room_participants
    FOR EACH ROW EXECUTE FUNCTION watch_room_status_notify();
