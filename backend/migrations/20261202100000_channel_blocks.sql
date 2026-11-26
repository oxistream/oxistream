-- Name and avatar are kept so the list can be shown without asking YouTube.
CREATE TABLE channel_blocks (
    user_id      UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    channel_id   TEXT        NOT NULL,
    channel_name TEXT        NOT NULL,
    -- JSON array of {url, width, height}.
    avatar       JSONB       NOT NULL DEFAULT '[]',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, channel_id)
);
