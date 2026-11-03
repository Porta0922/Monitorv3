-- Read-only projection: keep session retention and event deduplication unchanged.
-- Durations here are application/person time, never machine uptime.
CREATE VIEW desktop_activity AS
SELECT id,device_id,app_name::text,COALESCE(window_title,'') AS window_title,duration_seconds,timestamp,created_at FROM activity_logs
UNION ALL
SELECT id,device_id,COALESCE(payload->>'app_name','Sin identificar'),COALESCE(payload->>'window_title',''),
 CASE WHEN payload->>'duration_seconds' ~ '^[0-9]{1,10}$' THEN (payload->>'duration_seconds')::bigint ELSE 0 END,
 timestamp,timestamp FROM session_events WHERE kind='activity';
CREATE INDEX session_events_desktop_latest ON session_events(device_id,principal,timestamp DESC,id DESC)
 WHERE kind='running_apps';
CREATE VIEW desktop_running_apps AS
WITH latest AS (
 SELECT DISTINCT ON(device_id,principal) id,device_id,timestamp,payload
 FROM session_events WHERE kind='running_apps' ORDER BY device_id,principal,timestamp DESC,id DESC
)
SELECT md5(latest.id::text || ':' || item.ordinality::text)::uuid AS id,latest.device_id,
 COALESCE(item.app->>'app_name','Sin identificar') AS app_name,item.app->>'primary_title' AS primary_title,
 CASE WHEN item.app->>'window_count' ~ '^[0-9]{1,6}$' THEN (item.app->>'window_count')::int ELSE 0 END AS window_count,
 item.app->>'exe_path' AS exe_path,item.app->>'exe_hash' AS exe_hash,latest.timestamp AS updated_at
FROM latest CROSS JOIN LATERAL jsonb_array_elements(CASE WHEN jsonb_typeof(payload->'apps')='array' THEN payload->'apps' ELSE '[]'::jsonb END) WITH ORDINALITY item(app,ordinality)
UNION ALL
SELECT id,device_id,app_name,primary_title,window_count,exe_path,exe_hash,updated_at FROM running_apps_current r
WHERE NOT EXISTS(SELECT 1 FROM latest WHERE latest.device_id=r.device_id);
