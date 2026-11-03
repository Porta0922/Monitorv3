CREATE TABLE session_activity_daily (
 device_id uuid NOT NULL REFERENCES devices(device_id),day date NOT NULL,principal text NOT NULL,session_id uuid NOT NULL,
 first_seen timestamptz NOT NULL,last_seen timestamptz NOT NULL,
 events bigint NOT NULL,
 available_samples bigint NOT NULL,
 unavailable_samples bigint NOT NULL,
 unknown_samples bigint NOT NULL,
 active_person_seconds bigint,
 idle_person_seconds bigint,
 keys_count bigint,
 mouse_moves_count bigint,
 clicks_count bigint,
 PRIMARY KEY(device_id,day,principal,session_id)
);
CREATE VIEW session_activity_combined AS
WITH parts AS (
        SELECT device_id,(timestamp AT TIME ZONE 'UTC')::date AS day,principal,session_id,min(timestamp) AS first_seen,max(timestamp) AS last_seen,count(*) AS events,
          count(*) FILTER(WHERE kind='input_summary' AND payload->>'input_available'='true') AS available_samples,
          count(*) FILTER(WHERE kind='input_summary' AND payload->>'input_available'='false') AS unavailable_samples,
          count(*) FILTER(WHERE kind='input_summary' AND COALESCE(payload->>'input_available','') NOT IN ('true','false')) AS unknown_samples,
          sum(CASE WHEN kind='input_summary' AND payload->>'input_available'='true' THEN (payload->>'active_seconds')::bigint END) AS active_person_seconds,
          sum(CASE WHEN kind='input_summary' AND payload->>'input_available'='true' THEN (payload->>'idle_seconds')::bigint END) AS idle_person_seconds,
          sum(CASE WHEN kind='input_summary' AND payload->>'input_available'='true' THEN (payload->>'keys_count')::bigint END) AS keys_count,
          sum(CASE WHEN kind='input_summary' AND payload->>'input_available'='true' THEN (payload->>'mouse_moves_count')::bigint END) AS mouse_moves_count,
          sum(CASE WHEN kind='input_summary' AND payload->>'input_available'='true' THEN (payload->>'clicks_count')::bigint END) AS clicks_count
,count(*) FILTER(WHERE kind='input_summary') AS detail_samples,0::bigint AS summary_samples
 FROM session_events GROUP BY device_id,(timestamp AT TIME ZONE 'UTC')::date,principal,session_id
 UNION ALL SELECT device_id,day,principal,session_id,first_seen,last_seen,events,available_samples,unavailable_samples,unknown_samples,active_person_seconds,idle_person_seconds,keys_count,mouse_moves_count,clicks_count ,0::bigint AS detail_samples,available_samples+unavailable_samples+unknown_samples AS summary_samples FROM session_activity_daily
)
SELECT device_id,day,principal,session_id,min(first_seen) AS first_seen,max(last_seen) AS last_seen,
sum(events)::bigint AS events,sum(available_samples)::bigint AS available_samples,sum(unavailable_samples)::bigint AS unavailable_samples,sum(unknown_samples)::bigint AS unknown_samples,sum(active_person_seconds)::bigint AS active_person_seconds,sum(idle_person_seconds)::bigint AS idle_person_seconds,sum(keys_count)::bigint AS keys_count,sum(mouse_moves_count)::bigint AS mouse_moves_count,sum(clicks_count)::bigint AS clicks_count
,sum(detail_samples)::bigint AS detail_samples,sum(summary_samples)::bigint AS summary_samples
FROM parts GROUP BY device_id,day,principal,session_id;
