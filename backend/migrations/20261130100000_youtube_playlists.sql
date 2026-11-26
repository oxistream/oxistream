-- YouTube playlists someone follows, with what the feed worker last saw of them.
CREATE TABLE youtube_playlists (
    id           TEXT PRIMARY KEY,
    -- NULL until the feed worker fetched the playlist once.
    title        TEXT,
    channel_id   TEXT,
    channel_name TEXT,
    -- JSON array of {url, width, height}.
    thumbnails   JSONB       NOT NULL DEFAULT '[]',
    -- As reported by YouTube; may exceed the stored items.
    video_count  BIGINT      CHECK (video_count >= 0),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Last successful fetch.
    fetched_at   TIMESTAMPTZ,
    -- Last fetch attempt, successful or not.
    checked_at   TIMESTAMPTZ,
    last_error   TEXT
);

CREATE TABLE playlist_follows (
    user_id     UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    playlist_id TEXT        NOT NULL REFERENCES youtube_playlists (id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Items first seen after this count as new for the user.
    seen_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, playlist_id)
);

CREATE INDEX playlist_follows_playlist_id_idx ON playlist_follows (playlist_id);

-- The first items of a followed playlist, replaced on every fetch.
CREATE TABLE youtube_playlist_items (
    playlist_id      TEXT        NOT NULL REFERENCES youtube_playlists (id) ON DELETE CASCADE,
    video_id         TEXT        NOT NULL,
    -- Zero-based, in playlist order.
    position         INTEGER     NOT NULL CHECK (position >= 0),
    title            TEXT        NOT NULL,
    channel_id       TEXT,
    channel_name     TEXT,
    duration_seconds INTEGER     CHECK (duration_seconds >= 0),
    -- NULL for items of the first fetch, so they never count as new.
    first_seen_at    TIMESTAMPTZ,
    PRIMARY KEY (playlist_id, video_id),
    CONSTRAINT youtube_playlist_items_position_key UNIQUE (playlist_id, position) DEFERRABLE INITIALLY DEFERRED
);
