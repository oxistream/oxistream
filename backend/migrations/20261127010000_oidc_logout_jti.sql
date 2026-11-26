-- `jti` of accepted back-channel logout tokens, kept until they could no longer be accepted.
CREATE TABLE oidc_logout_jti (
    jti        TEXT PRIMARY KEY,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX oidc_logout_jti_expires_at_idx ON oidc_logout_jti (expires_at);
