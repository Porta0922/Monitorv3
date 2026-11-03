ALTER TABLE devices ADD COLUMN group_name text NOT NULL DEFAULT '';
ALTER TABLE devices ADD COLUMN desired_version text;
CREATE INDEX devices_group_name ON devices(group_name);
CREATE INDEX devices_last_seen_id ON devices(last_seen DESC,device_id);
CREATE TABLE administration_policy (
    scope text PRIMARY KEY,
    policy jsonb,
    revision bigint NOT NULL DEFAULT 1,
    updated_by text NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);
