CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT uuidv7(),
    username      TEXT        NOT NULL,
    -- NULL for accounts that can only sign in via OIDC.
    password_hash TEXT,
    role          TEXT        NOT NULL DEFAULT 'user' CHECK (role IN ('user', 'admin')),
    disabled      BOOLEAN     NOT NULL DEFAULT false,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX users_username_lower_key ON users (lower(username));

CREATE TABLE sessions (
    -- SHA-256 of the session token; the token itself is never stored.
    id           BYTEA PRIMARY KEY CHECK (octet_length(id) = 32),
    user_id      UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL,
    user_agent   TEXT,
    ip           INET,
    oidc_sid     TEXT
);

CREATE INDEX sessions_user_id_idx ON sessions (user_id);
CREATE INDEX sessions_expires_at_idx ON sessions (expires_at);
CREATE INDEX sessions_oidc_sid_idx ON sessions (oidc_sid) WHERE oidc_sid IS NOT NULL;
