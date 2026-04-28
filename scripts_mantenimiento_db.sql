-- =============================================================================
-- SQL SCRIPT: LIMPIEZA DE BASE DE DATOS Y ELIMINACIÓN DE DISPOSITIVOS DE PRUEBA
-- PROYECTO: ActivityMonitor-Enterprise-v3
-- OBJETIVO: Eliminar datos basura (nulos, desconocidos, duplicados) y dispositivos de prueba.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- PARTE 1: IDENTIFICACIÓN Y ELIMINACIÓN DE DISPOSITIVO DE PRUEBA
-- -----------------------------------------------------------------------------

-- 1.1 Listar todos los dispositivos registrados para identificar el ID del de prueba
-- Ejecuta esta consulta y copia el 'device_id' del dispositivo que quieres eliminar.
SELECT device_id, hostname, nickname, last_seen, created_at 
FROM devices 
ORDER BY last_seen DESC;

-- 1.2 ELIMINAR DISPOSITIVO ESPECÍFICO (Reemplaza el UUID con el que identificaste arriba)
-- Nota: Si la BD fue creada con los scripts de migración originales, el borrado será en cascada.
-- Si no, estas consultas aseguran que se borre todo lo relacionado.

/* 
-- DESCOMENTA Y REEMPLAZA EL UUID PARA EJECUTAR:
DO $$
DECLARE
    target_id UUID := 'AQUÍ_EL_UUID_DEL_DISPOSITIVO'; -- <--- REEMPLAZA ESTO
BEGIN
    -- Borrar de tablas hijas (por si no hay ON DELETE CASCADE)
    DELETE FROM activity_logs WHERE device_id = target_id;
    DELETE FROM usb_events WHERE device_id = target_id;
    DELETE FROM wifi_events WHERE device_id = target_id;
    DELETE FROM inventory WHERE device_id = target_id;
    DELETE FROM input_activity_metrics WHERE device_id = target_id;
    DELETE FROM node_resource_metrics WHERE device_id = target_id;
    DELETE FROM security_alerts WHERE device_id = target_id;
    DELETE FROM security_events WHERE device_id = target_id;
    DELETE FROM process_termination_attempts WHERE device_id = target_id;
    DELETE FROM running_apps_current WHERE device_id = target_id;
    
    -- Finalmente borrar el dispositivo
    DELETE FROM devices WHERE device_id = target_id;
    
    RAISE NOTICE 'Dispositivo % y todos sus datos asociados han sido eliminados.', target_id;
END $$;
*/


-- -----------------------------------------------------------------------------
-- PARTE 2: LIMPIEZA DE DATOS BASURA (ANOMALÍAS)
-- -----------------------------------------------------------------------------

BEGIN; -- Iniciar transacción para seguridad

-- 2.1 Eliminar logs de actividad sin información útil (Apps desconocidas o sin título)
DELETE FROM activity_logs
WHERE (
    app_name IS NULL 
    OR TRIM(app_name) = '' 
    OR LOWER(TRIM(app_name)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
) AND (
    window_title IS NULL 
    OR TRIM(window_title) = '' 
    OR LOWER(TRIM(window_title)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
);

-- 2.2 Eliminar logs con duración inválida (0 o negativa)
DELETE FROM activity_logs
WHERE duration_seconds <= 0;

-- 2.3 Eliminar ruido de USB (eventos que no son realmente USBs externos, ej: discos internos SATA/NVMe)
DELETE FROM usb_events
WHERE (hardware_id NOT ILIKE 'USBSTOR%') 
  AND (device_name ~* 'nvme|sata|raid|scsi');

-- 2.4 Eliminar eventos USB sin identificadores válidos
DELETE FROM usb_events
WHERE (serial_number IS NULL OR TRIM(serial_number) = '' OR LOWER(TRIM(serial_number)) = 'unknown')
  AND (hardware_id IS NULL OR TRIM(hardware_id) = '' OR LOWER(TRIM(hardware_id)) = 'unknown');

-- 2.5 Eliminar eventos de seguridad con MITRE technique inválido o erróneo
DELETE FROM security_events
WHERE mitre_technique IS NULL 
   OR TRIM(mitre_technique) = '' 
   OR LOWER(TRIM(mitre_technique)) IN ('unknown', 'n/a', 'none', 'error', 'null');

COMMIT; -- Confirmar cambios


-- -----------------------------------------------------------------------------
-- PARTE 3: ELIMINACIÓN DE DUPLICADOS (DEDUPING)
-- -----------------------------------------------------------------------------

BEGIN;

-- 3.1 Eliminar duplicados exactos en activity_logs
-- Mantiene el registro más antiguo (por id) y borra los demás que tengan misma fecha, dispositivo, app y título.
DELETE FROM activity_logs a
USING (
    SELECT id, 
           ROW_NUMBER() OVER (
               PARTITION BY device_id, timestamp, app_name, window_title, duration_seconds 
               ORDER BY created_at ASC
           ) as row_num
    FROM activity_logs
) b
WHERE a.id = b.id AND b.row_num > 1;

-- 3.2 Eliminar duplicados exactos en usb_events
DELETE FROM usb_events u
USING (
    SELECT id, 
           ROW_NUMBER() OVER (
               PARTITION BY device_id, timestamp, action, hardware_id, serial_number 
               ORDER BY created_at ASC
           ) as row_num
    FROM usb_events
) b
WHERE u.id = b.id AND b.row_num > 1;

-- 3.3 Eliminar duplicados en security_events
DELETE FROM security_events s
USING (
    SELECT id, 
           ROW_NUMBER() OVER (
               PARTITION BY device_id, timestamp, query_name, event_fingerprint 
               ORDER BY created_at ASC
           ) as row_num
    FROM security_events
) b
WHERE s.id = b.id AND b.row_num > 1;

COMMIT;

-- -----------------------------------------------------------------------------
-- PARTE 4: MANTENIMIENTO OPCIONAL (DATOS ANTIGUOS)
-- -----------------------------------------------------------------------------

-- Si quieres borrar logs de hace más de 30 días para liberar espacio, 
-- puedes usar esta consulta (DESCOMENTAR PARA USAR):

-- DELETE FROM activity_logs WHERE timestamp < NOW() - INTERVAL '30 days';
-- DELETE FROM node_resource_metrics WHERE timestamp < NOW() - INTERVAL '30 days';
-- DELETE FROM input_activity_metrics WHERE timestamp < NOW() - INTERVAL '30 days';

-- -----------------------------------------------------------------------------
-- PARTE 5: LIMPIEZA TOTAL DE SEGURIDAD (¡CUIDADO!)
-- -----------------------------------------------------------------------------

-- Si deseas vaciar completamente las tablas de seguridad (Alertas y Eventos) 
-- ejecuta las siguientes líneas. Esto borrará TODO el historial de seguridad.

/*
BEGIN;
-- Vaciar alertas de seguridad
TRUNCATE TABLE security_alerts RESTART IDENTITY CASCADE;

-- Vaciar eventos de seguridad (MITRE, etc)
TRUNCATE TABLE security_events RESTART IDENTITY CASCADE;

-- Vaciar intentos de terminación de procesos
TRUNCATE TABLE process_termination_attempts RESTART IDENTITY CASCADE;

COMMIT;
*/


-- -----------------------------------------------------------------------------
-- PARTE 6: REINICIO TOTAL (ELIMINAR TODOS LOS DISPOSITIVOS Y DATOS)
-- -----------------------------------------------------------------------------

-- ¡ATENCIÓN! Este bloque eliminará absolutamente TODOS los dispositivos registrados
-- y toda su actividad histórica. Úsalo solo para un reinicio total de pruebas.
-- Al ejecutar esto, la base de datos quedará en 0.

/* 
DO $$
BEGIN
    -- Borrar datos de todas las tablas principales
    TRUNCATE TABLE 
        activity_logs,
        usb_events,
        wifi_events,
        inventory,
        input_activity_metrics,
        node_resource_metrics,
        security_alerts,
        security_events,
        process_termination_attempts,
        running_apps_current,
        devices
    RESTART IDENTITY CASCADE;

    RAISE NOTICE 'REINICIO COMPLETADO: Todos los dispositivos y datos asociados han sido eliminados.';
END $$;
*/
