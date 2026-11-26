-- SponsorBlock preferences; users without a row, and keys missing from `settings`, use the defaults.
CREATE TABLE user_sponsorblock_settings (
    user_id    UUID        PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    settings   JSONB       NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
