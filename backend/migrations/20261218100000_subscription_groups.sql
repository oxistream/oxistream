-- Groups of subscribed channels, like NewPipe's channel groups.
CREATE TABLE subscription_groups (
    id           UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id      UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name         TEXT        NOT NULL,
    -- Zero-based order of the user's groups.
    position     INTEGER     NOT NULL CHECK (position >= 0),
    show_on_home BOOLEAN     NOT NULL DEFAULT false,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (id, user_id)
);

CREATE UNIQUE INDEX subscription_groups_user_name_idx ON subscription_groups (user_id, lower(name));

-- A channel can be in several groups; unsubscribing removes it from all of them.
CREATE TABLE subscription_group_channels (
    group_id   UUID NOT NULL,
    user_id    UUID NOT NULL,
    channel_id TEXT NOT NULL,
    PRIMARY KEY (group_id, channel_id),
    FOREIGN KEY (group_id, user_id) REFERENCES subscription_groups (id, user_id) ON DELETE CASCADE,
    FOREIGN KEY (user_id, channel_id) REFERENCES subscriptions (user_id, channel_id) ON DELETE CASCADE
);

CREATE INDEX subscription_group_channels_subscription_idx
    ON subscription_group_channels (user_id, channel_id);
