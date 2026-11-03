SET timezone='UTC';
INSERT INTO devices(device_id,hostname,last_seen) VALUES('00000000-0000-0000-0000-000000000001','SYNTHETIC',now());
DO $$
DECLARE d uuid:='00000000-0000-0000-0000-000000000001'; a uuid:=gen_random_uuid(); b uuid:=gen_random_uuid(); day_time timestamptz:=date_trunc('day',now())+interval '12 hours';
BEGIN
 INSERT INTO running_apps_current(id,device_id,app_name,updated_at) VALUES(gen_random_uuid(),d,'legacy',now());
 INSERT INTO session_events VALUES
 (gen_random_uuid(),d,'same-user',a,'running_apps',now()-interval '20 seconds','{"apps":[{"app_name":"first","window_count":1}]}'),
 (gen_random_uuid(),d,'same-user',b,'running_apps',now()-interval '5 minutes','{"apps":[{"app_name":"second","window_count":2}]}');
 IF (SELECT count(*) FROM desktop_running_apps WHERE device_id=d)<>2 THEN RAISE EXCEPTION 'Lost parallel desktop or duplicated legacy'; END IF;
 IF NOT (SELECT stale FROM desktop_running_apps WHERE app_name='second') THEN RAISE EXCEPTION 'Old report presented fresh'; END IF;
 IF (SELECT principal FROM desktop_running_apps WHERE app_name='first')<>'same-user' THEN RAISE EXCEPTION 'Missing attribution'; END IF;
 INSERT INTO session_events VALUES
 (gen_random_uuid(),d,'same-user',a,'running_apps',now()-interval '10 seconds','{"apps":[]}'),
 (gen_random_uuid(),d,'same-user',a,'running_apps',now()-interval '30 seconds','{"apps":[{"app_name":"late-old"}]}');
 IF (SELECT count(*) FROM desktop_running_apps WHERE device_id=d)<>1 OR EXISTS(SELECT FROM desktop_running_apps WHERE app_name IN ('first','late-old','legacy')) THEN RAISE EXCEPTION 'Empty/out-of-order snapshot resurrects windows'; END IF;
 INSERT INTO session_events VALUES
 (gen_random_uuid(),d,'same-user',a,'input_summary',day_time,'{"input_available":true,"active_seconds":10,"idle_seconds":20,"keys_count":5,"mouse_moves_count":6,"clicks_count":1}'),
 (gen_random_uuid(),d,'same-user',b,'input_summary',day_time,'{"input_available":false,"active_seconds":30,"idle_seconds":30,"keys_count":999,"mouse_moves_count":999,"clicks_count":999}'),
 (gen_random_uuid(),d,'same-user',b,'input_summary',day_time+interval '1 minute','{"input_available":true,"active_seconds":4,"idle_seconds":6,"keys_count":2,"mouse_moves_count":3,"clicks_count":2}');
END $$;
