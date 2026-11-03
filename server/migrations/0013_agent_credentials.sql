CREATE TABLE agent_credentials (
    credential_id UUID PRIMARY KEY,
    device_id UUID NOT NULL REFERENCES devices(device_id),
    token_hash CHAR(64) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    CHECK (expires_at > created_at)
);
CREATE INDEX agent_credentials_device_idx ON agent_credentials(device_id);
