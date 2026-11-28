-- Who besides the owner may control playback and the queue, and who may add videos.
ALTER TABLE watch_rooms
    ADD COLUMN control_permission TEXT NOT NULL DEFAULT 'everyone'
        CHECK (control_permission IN ('everyone', 'host')),
    ADD COLUMN add_permission TEXT NOT NULL DEFAULT 'everyone'
        CHECK (add_permission IN ('everyone', 'host'));

-- Participant key of whoever queued an entry, who may always remove it; NULL for autoplay.
ALTER TABLE watch_room_queue ADD COLUMN added_key TEXT;

-- The latest chat messages of a room; they go with it. `author_key` is the participant key,
-- `name` the display name at the time.
CREATE TABLE watch_room_messages (
    id         BIGINT      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    room_id    TEXT        NOT NULL REFERENCES watch_rooms (id) ON DELETE CASCADE,
    author_key TEXT        NOT NULL,
    name       TEXT        NOT NULL,
    body       TEXT        NOT NULL CHECK (char_length(body) BETWEEN 1 AND 500),
    sent_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX watch_room_messages_room_id_idx ON watch_room_messages (room_id, id);

-- Payload: `<room> <id> <sent ms> <author key>`, then the name and the body on their own lines.
-- Neither contains line breaks.
CREATE FUNCTION watch_room_message_notify() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_notify('watch_room_chat', concat_ws(' ', NEW.room_id, NEW.id,
        floor(extract(epoch FROM NEW.sent_at) * 1000)::bigint, NEW.author_key)
        || E'\n' || NEW.name || E'\n' || NEW.body);
    RETURN NULL;
END
$$;

CREATE TRIGGER watch_room_message_notify
    AFTER INSERT ON watch_room_messages
    FOR EACH ROW EXECUTE FUNCTION watch_room_message_notify();
