ALTER TABLE agent_credentials ADD COLUMN predecessor UUID UNIQUE REFERENCES agent_credentials(credential_id);
CREATE TABLE agent_enrollments (
 enrollment_id UUID PRIMARY KEY,
 device_id UUID NOT NULL REFERENCES devices(device_id),
 code_hash CHAR(64) NOT NULL UNIQUE,
 expires_at TIMESTAMPTZ NOT NULL,
 credential_id UUID REFERENCES agent_credentials(credential_id),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX agent_enrollment_expiry ON agent_enrollments(expires_at);
