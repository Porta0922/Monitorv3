-- Aggregate-only read, no usernames, tokens, titles or raw security payloads.
BEGIN READ ONLY;
SET LOCAL statement_timeout = '10s';
SELECT json_build_object(
 'failed_logins', (SELECT count(*) FROM audit_events WHERE action='auth.login.failed' AND created_at>now()-interval '15 minutes'),
 'limited_accounts', (SELECT count(*) FROM audit_events WHERE action='auth.login.limited' AND created_at>now()-interval '15 minutes'),
 'denied_requests', (SELECT count(*) FROM audit_events WHERE action='access.denied' AND created_at>now()-interval '15 minutes'),
 'csrf_rejections', (SELECT count(*) FROM audit_events WHERE action='auth.csrf.rejected' AND created_at>now()-interval '15 minutes'),
 'security_high', (SELECT count(*) FROM security_events WHERE lower(severity) IN ('high','critical') AND created_at>now()-interval '15 minutes'),
 'maintenance_failures', (SELECT count(*) FROM maintenance_job_status WHERE last_failure_at>COALESCE(last_success_at,'epoch'::timestamptz)),
 'maintenance_age', (SELECT COALESCE(EXTRACT(EPOCH FROM now()-max(last_success_at)),2147483647) FROM maintenance_status WHERE job='metrics-retention'),
 'database_bytes', pg_database_size(current_database())
);
COMMIT;
