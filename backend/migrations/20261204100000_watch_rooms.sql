CREATE TABLE watch_rooms (
    -- Random, unguessable 128-bit ID; anyone with the link may join.
    id               TEXT        PRIMARY KEY,
    owner_id         UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- The queue entry being played; NULL once the queue has run out.
    current_entry_id BIGINT,
    playing          BOOLEAN     NOT NULL DEFAULT false,
    -- Playback position at position_at; while playing, it advances with the clock.
    position_ms      BIGINT      NOT NULL DEFAULT 0 CHECK (position_ms >= 0),
    position_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Bumped on every change, including participants joining and leaving.
    revision         BIGINT      NOT NULL DEFAULT 0,
    -- Participant keys the owner removed.
    banned           TEXT[]      NOT NULL DEFAULT '{}',
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Last time anyone was in the room; rooms expire after a day without participants.
    active_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX watch_rooms_owner_id_idx ON watch_rooms (owner_id, created_at DESC);
CREATE INDEX watch_rooms_active_at_idx ON watch_rooms (active_at);

-- The current entry and the upcoming ones; played entries are deleted.
CREATE TABLE watch_room_queue (
    id       BIGINT      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    room_id  TEXT        NOT NULL REFERENCES watch_rooms (id) ON DELETE CASCADE,
    video_id TEXT        NOT NULL REFERENCES videos (id),
    added_by TEXT        NOT NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX watch_room_queue_room_id_idx ON watch_room_queue (room_id, id);

-- One row per open connection; replicas refresh seen_at while it lasts.
CREATE TABLE watch_room_participants (
    id           UUID        PRIMARY KEY,
    room_id      TEXT        NOT NULL REFERENCES watch_rooms (id) ON DELETE CASCADE,
    -- `u:<user id>` or `g:<SHA-256 of the guest key>`; bans apply to it.
    key          TEXT        NOT NULL,
    user_id      UUID        REFERENCES users (id) ON DELETE CASCADE,
    display_name TEXT        NOT NULL,
    joined_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    seen_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX watch_room_participants_room_id_idx ON watch_room_participants (room_id, joined_at);
CREATE INDEX watch_room_participants_seen_at_idx ON watch_room_participants (seen_at);

-- Every replica listens on `watch_rooms` and reloads the room named in the payload.
CREATE FUNCTION watch_rooms_notify() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_notify('watch_rooms', to_jsonb(COALESCE(NEW, OLD)) ->> TG_ARGV[0]);
    RETURN NULL;
END
$$;

CREATE TRIGGER watch_rooms_notify
    AFTER UPDATE OF revision OR DELETE ON watch_rooms
    FOR EACH ROW EXECUTE FUNCTION watch_rooms_notify('id');

-- Covers participants removed by a cascade, e.g. a deleted account.
CREATE TRIGGER watch_room_participants_notify
    AFTER DELETE ON watch_room_participants
    FOR EACH ROW EXECUTE FUNCTION watch_rooms_notify('room_id');
