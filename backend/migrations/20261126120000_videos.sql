CREATE TABLE videos (
    id               TEXT PRIMARY KEY,
    title            TEXT        NOT NULL,
    channel_id       TEXT        NOT NULL,
    channel_name     TEXT        NOT NULL,
    duration_seconds INTEGER     CHECK (duration_seconds >= 0),
    published_at     TIMESTAMPTZ,
    thumbnail_url    TEXT,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX videos_channel_id_idx ON videos (channel_id);
