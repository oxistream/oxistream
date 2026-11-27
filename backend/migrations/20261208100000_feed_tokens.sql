-- Secret of a user's personal Atom feed URL; at most one per user.
CREATE TABLE feed_tokens (
    user_id    UUID PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    -- SHA-256 of the token; the token itself is never stored.
    token_hash BYTEA       NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
