# Conservación de seguridad

La migración 0006 agrega investigaciones que protegen los eventos de un equipo dentro de un intervalo UTC inclusivo. El alcance incluye eventos recibidos después de abrir la investigación. No recupera datos ya eliminados.

API bajo el router autenticado de seguridad:
- GET /security/investigations?device_id=UUID&limit=50&offset=0: listado con has_more.
- POST /security/investigations: device_id, title, reason, event_from, event_to (RFC3339).
- POST /security/investigations/:id/release: reason obligatorio. Conserva la investigación y su motivo de liberación; no permite sobrescribir una liberación previa.

La apertura comparte el bloqueo transaccional del mantenimiento. Una vez confirmada, las siguientes tandas excluyen los eventos protegidos. La liberación habilita su limpieza en una tanda posterior si superan la retención. Dos investigaciones superpuestas mantienen protección hasta liberar ambas.

SECURITY_EVENT_RETENTION_DAYS=0 desactiva esta tarea. Para habilitarla, configurar un plazo de 1 a 3650 días y HISTORY_RETENTION_ENABLED=true. No se activa automáticamente. Aplica el límite de lote, timeout y estado de mantenimiento existentes. Las investigaciones no se eliminan con los eventos.

El dashboard incluye apertura/listado/liberación con motivo, filtro por equipo y paginación. La migración 0008 registra created_by/released_by desde el JWT; los registros anteriores sin actor se muestran como no registrados. Por ahora se aplica la autenticación administrativa existente. Pendiente: modelo multiusuario con permisos por rol. Implementación sin ejecutar migraciones ni pruebas; comprobación integrada reservada a P4.
