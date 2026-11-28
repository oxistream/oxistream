-- Subscription feed filters, kept per user.
ALTER TABLE user_settings
    ADD COLUMN feed_hide_watched BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN feed_hide_shorts  BOOLEAN NOT NULL DEFAULT false;

-- Videos a user is not interested in, left out of the subscription feed and home rows.
CREATE TABLE hidden_videos (
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    video_id   TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, video_id)
);
