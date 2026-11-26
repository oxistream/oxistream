CREATE TABLE playlists (
    -- Random, unguessable public ID; unlisted playlists are shared by it.
    id            TEXT PRIMARY KEY,
    user_id       UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    title         TEXT        NOT NULL,
    description   TEXT        NOT NULL DEFAULT '',
    visibility    TEXT        NOT NULL DEFAULT 'private' CHECK (visibility IN ('private', 'unlisted')),
    -- Where the owner left off playing the playlist.
    last_video_id TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX playlists_user_id_idx ON playlists (user_id, updated_at DESC);

CREATE TABLE playlist_items (
    playlist_id TEXT        NOT NULL REFERENCES playlists (id) ON DELETE CASCADE,
    video_id    TEXT        NOT NULL REFERENCES videos (id),
    -- Dense, zero-based; renumbered in the same transaction on every change.
    position    INTEGER     NOT NULL CHECK (position >= 0),
    added_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (playlist_id, video_id),
    CONSTRAINT playlist_items_position_key UNIQUE (playlist_id, position) DEFERRABLE INITIALLY DEFERRED
);

CREATE INDEX playlist_items_video_id_idx ON playlist_items (video_id);
