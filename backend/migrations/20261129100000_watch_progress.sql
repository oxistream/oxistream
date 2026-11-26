-- One row per user once a setting differs from its default.
CREATE TABLE user_settings (
    user_id        UUID PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    history_paused BOOLEAN     NOT NULL DEFAULT false,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Doubles as watch history and resume point. No foreign key to `videos`, since progress may
-- arrive before the video's metadata is cached.
CREATE TABLE watch_progress (
    user_id          UUID             NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    video_id         TEXT             NOT NULL,
    position_seconds DOUBLE PRECISION NOT NULL CHECK (position_seconds >= 0),
    duration_seconds DOUBLE PRECISION NOT NULL CHECK (duration_seconds > 0),
    finished         BOOLEAN          NOT NULL DEFAULT false,
    updated_at       TIMESTAMPTZ      NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, video_id)
);

CREATE INDEX watch_progress_user_updated_idx
    ON watch_progress (user_id, updated_at DESC, video_id DESC);
