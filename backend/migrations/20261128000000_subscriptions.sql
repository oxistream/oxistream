CREATE TABLE channels (
    id               TEXT PRIMARY KEY,
    -- NULL until the feed worker fetched the channel once.
    name             TEXT,
    handle           TEXT,
    -- JSON array of {url, width, height}.
    avatar           JSONB       NOT NULL DEFAULT '[]',
    subscriber_count BIGINT      CHECK (subscriber_count >= 0),
    verified         BOOLEAN     NOT NULL DEFAULT false,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Last successful fetch.
    fetched_at       TIMESTAMPTZ,
    -- Last fetch attempt, successful or not.
    checked_at       TIMESTAMPTZ,
    last_error       TEXT
);

CREATE TABLE subscriptions (
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    channel_id TEXT        NOT NULL REFERENCES channels (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, channel_id)
);

CREATE INDEX subscriptions_channel_id_idx ON subscriptions (channel_id);

CREATE TABLE channel_videos (
    video_id         TEXT PRIMARY KEY,
    channel_id       TEXT        NOT NULL REFERENCES channels (id) ON DELETE CASCADE,
    title            TEXT        NOT NULL,
    duration_seconds INTEGER     CHECK (duration_seconds >= 0),
    published_at     TIMESTAMPTZ,
    published_text   TEXT,
    -- JSON array of {url, width, height}.
    thumbnails       JSONB       NOT NULL DEFAULT '[]',
    view_count       BIGINT      CHECK (view_count >= 0),
    is_live          BOOLEAN     NOT NULL DEFAULT false,
    is_upcoming      BOOLEAN     NOT NULL DEFAULT false,
    is_short         BOOLEAN     NOT NULL DEFAULT false,
    first_seen_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    sort_at          TIMESTAMPTZ GENERATED ALWAYS AS (COALESCE(published_at, first_seen_at)) STORED
);

CREATE INDEX channel_videos_channel_sort_idx ON channel_videos (channel_id, sort_at DESC, video_id DESC);
CREATE INDEX channel_videos_sort_idx ON channel_videos (sort_at DESC, video_id DESC);
