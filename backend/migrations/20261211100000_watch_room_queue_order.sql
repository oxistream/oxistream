-- Order of the queue. New entries take the next value; reordering and shuffling only swap the
-- positions of the upcoming entries among each other.
CREATE SEQUENCE watch_room_queue_position_seq;

ALTER TABLE watch_room_queue ADD COLUMN position BIGINT;
UPDATE watch_room_queue SET position = id;
SELECT setval('watch_room_queue_position_seq', COALESCE(max(id), 0) + 1, false)
FROM watch_room_queue;

ALTER TABLE watch_room_queue
    ALTER COLUMN position SET DEFAULT nextval('watch_room_queue_position_seq'),
    ALTER COLUMN position SET NOT NULL;
ALTER SEQUENCE watch_room_queue_position_seq OWNED BY watch_room_queue.position;

DROP INDEX watch_room_queue_room_id_idx;
CREATE INDEX watch_room_queue_room_id_idx ON watch_room_queue (room_id, position);
