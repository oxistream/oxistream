-- Played entries stay as the room's history instead of being deleted. `played` orders them,
-- newest last; NULL marks the current and upcoming entries.
CREATE SEQUENCE watch_room_played_seq;

ALTER TABLE watch_room_queue ADD COLUMN played BIGINT;

CREATE INDEX watch_room_queue_played_idx ON watch_room_queue (room_id, played)
    WHERE played IS NOT NULL;
