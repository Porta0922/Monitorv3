WITH grouped AS (SELECT * FROM session_activity_combined WHERE device_id=$1 AND day=$2::date
      ) SELECT jsonb_build_object('items',COALESCE((SELECT jsonb_agg(to_jsonb(p) ORDER BY p.principal,p.session_id) FROM
        (SELECT * FROM grouped ORDER BY principal,session_id LIMIT $4 OFFSET $3) p),'[]'::jsonb),
        'totals',(SELECT jsonb_build_object('active_person_seconds',sum(active_person_seconds),'idle_person_seconds',sum(idle_person_seconds),
          'keys_count',sum(keys_count),'mouse_moves_count',sum(mouse_moves_count),'clicks_count',sum(clicks_count),
          'available_samples',COALESCE(sum(available_samples),0),'unavailable_samples',COALESCE(sum(unavailable_samples),0),
          'unknown_samples',COALESCE(sum(unknown_samples),0)) FROM grouped),
        'total',(SELECT count(*) FROM grouped),'day',$2::date,'timezone','UTC')
