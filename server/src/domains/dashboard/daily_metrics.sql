-- One MVCC snapshot combines disjoint raw and compacted rows, including late data.
WITH resource_parts AS (
    SELECT day, samples, cpu_sum, cpu_max, memory_percent_sum, memory_percent_max,
           memory_used_mb_sum, 0::bigint AS detail_samples, samples AS summary_samples
    FROM resource_metrics_daily WHERE device_id=$1 AND day BETWEEN $2 AND $3
    UNION ALL
    SELECT (timestamp AT TIME ZONE 'UTC')::date, count(*), sum(cpu_percent), max(cpu_percent),
           sum(memory_percent), max(memory_percent), sum(memory_used_mb), count(*), 0::bigint
    FROM node_resource_metrics
    WHERE device_id=$1 AND timestamp >= ($2::date::timestamp AT TIME ZONE 'UTC')
      AND timestamp < (($3::date+1)::timestamp AT TIME ZONE 'UTC')
    GROUP BY 1
), resources AS (
    SELECT day, sum(samples)::bigint AS samples,
           sum(cpu_sum)/nullif(sum(samples),0) AS cpu_average, max(cpu_max) AS cpu_peak,
           sum(memory_percent_sum)/nullif(sum(samples),0) AS memory_average,
           max(memory_percent_max) AS memory_peak,
           sum(memory_used_mb_sum)/nullif(sum(samples),0) AS memory_used_mb_average,
           sum(detail_samples)::bigint AS detail_samples, sum(summary_samples)::bigint AS summary_samples
    FROM resource_parts GROUP BY day
), inputs AS (
 SELECT day,sum(available_samples+unavailable_samples+unknown_samples)::bigint AS samples,
 sum(available_samples)::bigint AS available_samples,sum(unavailable_samples)::bigint AS unavailable_samples,
 sum(active_person_seconds)::bigint AS active_seconds,sum(idle_person_seconds)::bigint AS idle_seconds,
 sum(keys_count)::bigint AS keys_count,sum(mouse_moves_count)::bigint AS mouse_moves_count,sum(clicks_count)::bigint AS clicks_count,
 sum(detail_samples)::bigint AS detail_samples,sum(summary_samples)::bigint AS summary_samples
 FROM session_activity_combined WHERE device_id=$1 AND day BETWEEN $2 AND $3 GROUP BY day
)

SELECT coalesce(r.day,i.day) AS day,
       coalesce(i.available_samples,0) AS input_available_samples,coalesce(i.unavailable_samples,0) AS input_unavailable_samples,coalesce(i.samples-i.available_samples-i.unavailable_samples,0) AS input_unknown_samples,
       coalesce(r.samples,0) AS resource_samples, coalesce(i.samples,0) AS input_samples,
       r.cpu_average, r.cpu_peak, r.memory_average, r.memory_peak, r.memory_used_mb_average,
       i.active_seconds, i.idle_seconds, i.keys_count, i.mouse_moves_count, i.clicks_count,
       CASE WHEN coalesce(r.detail_samples,0)+coalesce(i.detail_samples,0) = 0 THEN 'summary'
            WHEN coalesce(r.summary_samples,0)+coalesce(i.summary_samples,0) = 0 THEN 'detail'
            ELSE 'mixed' END AS source
FROM resources r FULL OUTER JOIN inputs i ON r.day=i.day
ORDER BY day DESC
