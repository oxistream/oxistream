-- The up-next queue of signed-in users. `revision` grows with every change so clients can
-- tell newer states from older ones.
CREATE TABLE queues (
    user_id    UUID        PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    revision   BIGINT      NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Metadata comes from the client and is only shown to the same user.
CREATE TABLE queue_entries (
    user_id          UUID    NOT NULL REFERENCES queues (user_id) ON DELETE CASCADE,
    key              TEXT    NOT NULL CHECK (key ~ '^[A-Za-z0-9_-]{1,64}$'),
    position         INTEGER NOT NULL,
    video_id         TEXT    NOT NULL,
    title            TEXT    NOT NULL,
    channel_id       TEXT,
    channel_name     TEXT,
    duration_seconds INTEGER,
    PRIMARY KEY (user_id, key)
);

CREATE INDEX queue_entries_position_idx ON queue_entries (user_id, position);
