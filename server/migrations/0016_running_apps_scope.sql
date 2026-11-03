-- Preserve each desktop independently. A report is evidence of observation,
-- not proof that a window is still open when the dashboard is queried.
CREATE OR REPLACE VIEW desktop_running_apps AS
WITH latest AS (
 SELECT DISTINCT ON(device_id,principal,session_id)
  id,device_id,principal,session_id,timestamp,payload
 FROM session_events WHERE kind='running_apps'
 ORDER BY device_id,principal,session_id,timestamp DESC,id DESC
)
SELECT md5(latest.id::text || ':' || item.ordinality::text)::uuid AS id,latest.device_id,
 COALESCE(item.app->>'app_name','Sin identificar') AS app_name,item.app->>'primary_title' AS primary_title,
 CASE WHEN item.app->>'window_count' ~ '^[0-9]{1,6}$' THEN (item.app->>'window_count')::int ELSE 0 END AS window_count,
 item.app->>'exe_path' AS exe_path,item.app->>'exe_hash' AS exe_hash,latest.timestamp AS updated_at,
 latest.principal,latest.session_id,'session'::text AS source,
 latest.timestamp < now()-interval '180 seconds' OR latest.timestamp > now()+interval '30 seconds' AS stale
FROM latest CROSS JOIN LATERAL jsonb_array_elements(CASE WHEN jsonb_typeof(payload->'apps')='array' THEN payload->'apps' ELSE '[]'::jsonb END) WITH ORDINALITY item(app,ordinality)
UNION ALL
SELECT id,device_id,app_name,primary_title,window_count,exe_path,exe_hash,updated_at,
 NULL::text,NULL::uuid,'legacy'::text,
 updated_at < now()-interval '180 seconds' OR updated_at > now()+interval '30 seconds'
FROM running_apps_current r
WHERE NOT EXISTS(SELECT 1 FROM latest WHERE latest.device_id=r.device_id);
