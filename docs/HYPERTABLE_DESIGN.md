# Decisión P0: particionado explícito

El despliegue P0 usa tablas PostgreSQL normales para nuevas instalaciones y
conserva las hypertables existentes. No convierte tablas durante el arranque.
Esta decisión evita modificar claves o reescribir grandes tablas implícitamente.

## Conversión futura de métricas

La primera conversión candidata es `node_resource_metrics` y
`input_activity_metrics`, particionadas por `timestamp` en intervalos de un día.
Sus claves primarias pasarían de `(id)` a `(timestamp, id)`; cualquier índice
UNIQUE debe incluir `timestamp`. Antes de convertir, hay que comprobar que no
existen claves foráneas externas que referencien solo `id` y que los escritores
no dependan de `ON CONFLICT (id)`. `processed_events` permanece como tabla normal
para conservar la deduplicación global por evento.

La conversión se hará en una migración optativa, fuera del arranque normal:
backup verificado, pausa de escritores/mantenimiento, transacción con
`lock_timeout`, claves compatibles y `create_hypertable(..., migrate_data => true)`.
Debe verificar recuentos y sumas antes/después. Si falla, la transacción revierte;
para revertir después de confirmar, restaurar el backup, no eliminar chunks.
No activar políticas `drop_chunks` sobre métricas sin que hayan sido agregadas.

## Compresión y retención

No se activa compresión de métricas en P0. El mantenimiento actual usa
`tableoid` y `ctid` para mover lotes; su funcionamiento con tablas normales no
certifica los chunks comprimidos. Antes de habilitar la conversión/compresión,
P1 debe probar agregación, eliminación acotada, rollback, datos tardíos y
concurrencia con chunks comprimidos. La compresión debe ir después de la ventana
de detalle o restringirse a resúmenes, según el resultado de esas pruebas.

Las tablas de dispositivos, comandos, alertas, auditoría, inventario actual y
resúmenes diarios se mantienen normales. No se convertirán solo por contener
una fecha. Las políticas antiguas de actividad/heatmaps permanecen sin cambios;
la prueba de actualización comprueba el caso de actividad comprimida.
