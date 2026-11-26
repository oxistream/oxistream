-- Accounts are identified by the provider's (issuer, subject), never by email.
CREATE TABLE user_identities (
    issuer     TEXT        NOT NULL,
    subject    TEXT        NOT NULL,
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (issuer, subject)
);

CREATE INDEX user_identities_user_id_idx ON user_identities (user_id);

-- Sent as `id_token_hint` for RP-initiated logout.
ALTER TABLE sessions ADD COLUMN oidc_id_token TEXT;
