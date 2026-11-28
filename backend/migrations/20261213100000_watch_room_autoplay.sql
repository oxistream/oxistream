-- When the queue runs out, autoplay appends `autoplay_video_id`, a related video of the entry
-- `autoplay_source`; a source without a video means nothing was found for it.
ALTER TABLE watch_rooms
    ADD COLUMN autoplay          BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN autoplay_source   BIGINT,
    ADD COLUMN autoplay_video_id TEXT REFERENCES videos (id);

ALTER TABLE watch_room_queue ADD COLUMN autoplay BOOLEAN NOT NULL DEFAULT false;

-- Every video a room has queued, so autoplay does not repeat any.
CREATE TABLE watch_room_seen (
    room_id  TEXT NOT NULL REFERENCES watch_rooms (id) ON DELETE CASCADE,
    video_id TEXT NOT NULL,
    PRIMARY KEY (room_id, video_id)
);

INSERT INTO watch_room_seen (room_id, video_id)
SELECT DISTINCT room_id, video_id FROM watch_room_queue;

CREATE FUNCTION watch_room_queue_seen() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO watch_room_seen (room_id, video_id) VALUES (NEW.room_id, NEW.video_id)
    ON CONFLICT DO NOTHING;
    -- A pick queued by hand would repeat.
    UPDATE watch_rooms SET autoplay_source = NULL, autoplay_video_id = NULL
    WHERE id = NEW.room_id AND autoplay_video_id = NEW.video_id;
    RETURN NULL;
END
$$;

CREATE TRIGGER watch_room_queue_seen
    AFTER INSERT ON watch_room_queue
    FOR EACH ROW EXECUTE FUNCTION watch_room_queue_seen();
