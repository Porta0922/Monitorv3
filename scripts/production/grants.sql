-- Applied by the migration owner, after each successful migration.
REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO monitor_app, monitor_maintenance;
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM monitor_app, monitor_maintenance;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM monitor_app, monitor_maintenance;
-- Only DML; no ownership, DDL, role management or superuser privileges.
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO monitor_app;
REVOKE ALL ON public._sqlx_migrations FROM monitor_app;
GRANT SELECT ON public._sqlx_migrations TO monitor_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO monitor_app;
GRANT SELECT, DELETE ON public.node_resource_metrics, public.input_activity_metrics,
 public.activity_logs, public.session_events, public.usb_events, public.wifi_events,
 public.audit_events, public.processed_events, public.security_events,
 public.inventory_changes, public.security_alerts TO monitor_maintenance;
-- Row locking in bounded retention batches needs UPDATE privilege.
GRANT UPDATE ON public.node_resource_metrics, public.input_activity_metrics,
 public.activity_logs, public.session_events, public.usb_events, public.wifi_events,
 public.audit_events, public.processed_events, public.security_events,
 public.inventory_changes, public.security_alerts TO monitor_maintenance;
GRANT SELECT ON public.security_investigations TO monitor_maintenance;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.resource_metrics_daily, public.input_metrics_daily,
 public.maintenance_status, public.maintenance_job_status TO monitor_maintenance;

GRANT SELECT, INSERT, UPDATE, DELETE ON public.session_activity_daily TO monitor_maintenance;
