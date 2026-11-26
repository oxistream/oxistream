-- Local only; never sent to YouTube.
CREATE TABLE video_likes (
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    video_id   TEXT        NOT NULL REFERENCES videos (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, video_id)
);

CREATE INDEX video_likes_user_id_idx ON video_likes (user_id, created_at DESC, video_id DESC);
CREATE INDEX video_likes_video_id_idx ON video_likes (video_id);
