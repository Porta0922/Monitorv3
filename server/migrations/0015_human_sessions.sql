CREATE TABLE human_sessions (
 session_id UUID PRIMARY KEY,
 token_hash CHAR(64) UNIQUE NOT NULL,
 username VARCHAR(100) NOT NULL,
 issued_role VARCHAR(16) NOT NULL CHECK (issued_role IN ('viewer','operator','admin')),
 account_version CHAR(64) NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 last_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
 expires_at TIMESTAMPTZ NOT NULL,
 revoked_at TIMESTAMPTZ
);
CREATE INDEX human_sessions_username ON human_sessions(username);
CREATE TABLE auth_login_limits (
 bucket CHAR(64) PRIMARY KEY,
 window_started TIMESTAMPTZ NOT NULL DEFAULT now(),
 attempts INTEGER NOT NULL
);
CREATE TABLE auth_mfa_steps (
 account_version CHAR(64) PRIMARY KEY,
 last_step BIGINT NOT NULL,
 updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX human_sessions_expiry ON human_sessions(expires_at);
CREATE INDEX auth_login_limits_window ON auth_login_limits(window_started);
