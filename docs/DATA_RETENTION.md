# Retención de métricas: primer incremento

## Ejecución

`docker compose up -d --build` inicia también `db-maintenance`, después del
healthcheck del servidor. Este proceso ejecuta un lote cada cinco minutos.
Las rutas SQL se montan de solo lectura; no dependen de que el volumen PostgreSQL
sea nuevo. Configuración en `.env`:

```dotenv
METRICS_RAW_RETENTION_DAYS=14
METRICS_SUMMARY_RETENTION_DAYS=365
MAINTENANCE_BATCH_SIZE=5000
MAINTENANCE_INTERVAL_SECONDS=300
```

El límite es por tabla y ejecución. Con estos valores cada tabla puede compactar
hasta 1.440.000 muestras/día si las ejecuciones completan en tiempo. La carga real,
el tamaño de los índices y el backlog deben medirse; no es una garantía de rendimiento.
Los cortes usan días completos UTC y el instante de medición, no el de recepción.

```sh
docker compose logs --tail=100 db-maintenance
docker compose run --rm db-maintenance --once
docker compose stop db-maintenance
```

`--once` procesa un lote, no necesariamente todo el backlog. Fallos SQL producen
salida no cero en ese modo. En modo periódico se registran y reintentan en el
siguiente intervalo. `restart: unless-stopped` permite recuperación del contenedor;
requiere que el motor Docker arranque con el servidor anfitrión.

## Garantías y consultas

Eliminación del detalle y acumulación del resumen ocurren en una sola transacción.
Si falla cualquiera de las tablas se revierte todo el lote. Un advisory lock evita
dos mantenimientos simultáneos. Los datos tardíos aún dentro de 365 días se suman
al resumen; los más antiguos se eliminan sin conservar resumen.

Los promedios se calculan con suma y cantidad de muestras, no promediando promedios.
Son promedios de muestras, no ponderados por duración. Se conservan máximos de
CPU/memoria, pero no nombres de procesos ni percentiles del detalle descartado.

```sql
SELECT device_id, day, samples, cpu_sum / samples AS cpu_avg, cpu_max,
       memory_percent_sum / samples AS memory_avg, memory_percent_max
FROM resource_metrics_daily ORDER BY day DESC;

SELECT * FROM maintenance_status;
SELECT min(timestamp), count(*) FROM node_resource_metrics;
```

Los resúmenes aparecen al vencer el detalle, no inmediatamente. La API/dashboard
actuales todavía no consultan estas tablas: consultas históricas de recursos y
actividad más antiguas que la ventana de detalle deberán usar los resúmenes.
Esto debe integrarse en la fase de API/dashboard antes del piloto de producción.

## Compatibilidad y límites

La implementación usa el esquema actual de tablas PostgreSQL del servidor.
No necesita TimescaleDB. Las pruebas locales de SQL usan PostgreSQL WASM (PGlite);
la compatibilidad con hypertables comprimidas existentes requiere un ensayo de
migración y no se presupone. Un error revierte el lote, no se oculta.

El viejo script intentaba comprimir tablas sin validar si eran hypertables,
usaba `date -d` sin argumento y no tenía una programación periódica efectiva.
Ahora no ejecutamos compresión, REINDEX ni VACUUM FULL automáticos. Autovacuum
recupera espacio para reutilización; DELETE no implica que el archivo en disco
se reduzca. La migración a particiones/chunks permitirá eliminar períodos enteros.

No se toca `processed_events`: limpiarlo antes de fijar el máximo de reenvío
permitiría duplicados. Tampoco se borran alertas abiertas ni inventario.
Las políticas Timescale antiguas, como la de `activity_logs`, siguen existiendo;
esta entrega no promete un límite global del tamaño de toda la base.

## Pruebas

```sh
cd tests/storage
npm ci
npm test
```

Verifican lotes pequeños, promedios, reintentos, datos tardíos, frontera UTC,
vencimiento, configuración inválida y rollback ante error de agregación.
PGlite no valida el daemon Docker, el wrapper psql ni concurrencia entre conexiones.

`tests/storage/integration.sh` prueba el wrapper psql, reintentos y rollback sobre
una base PostgreSQL vacía y descartable. El workflow `Storage retention` lo ejecuta
con PostgreSQL 15. No apuntar esa prueba a una base de producción.

Validación actualizada el 2026-10-23: 4 pruebas PGlite aprobadas, sintaxis Bash
y Compose aprobados. Integración real PostgreSQL 16 Docker aprobada (lotes,
reintentos y rollback), ejecución periódica y exclusión por advisory lock aprobadas.
El dashboard compila y sirve HTTP. PostgreSQL 15/TimescaleDB, RabbitMQ y Redis del
Compose arrancan saludables con volúmenes nuevos. El bloqueo de compilación se
resolvió fijando Rust 1.99.1 y Cargo.lock. Se verificó la integración con una
métrica sintética enviada por RabbitMQ y consultada a través de Nginx/API,
incluyendo reenvío duplicado y persistencia después de reiniciar DB/API.
Ver STACK_VALIDATION.md y TODO.md para límites y pasos restantes. No se ejecutó
el workflow remoto ni se publicaron cambios en GitHub.


## P1 en desarrollo — 2026-10-24

Implementación nueva en código, sin desplegar ni validar integralmente. Las pruebas
se posponen por indicación del usuario. No habilitar aún HISTORY_RETENTION_ENABLED.

La migración 0002 incorpora almacenamiento de resúmenes; la API
`GET /api/resources/:device_id/daily?from=YYYY-MM-DD&to=YYYY-MM-DD` combina detalle y
resumen en una consulta consistente, hasta 366 días inclusivos UTC. CPU/RAM son
promedios por muestra; actividad e inactividad son duraciones acumuladas.

La migración 0003 incorpora el estado de limpieza. El daemon mantiene la
compactación de métricas y añade tareas independientes si se habilitan explícitamente:

| Variable | Días predeterminados | Fecha de corte |
| --- | ---: | --- |
| ACTIVITY_RETENTION_DAYS | 30 | timestamp |
| USB_RETENTION_DAYS | 90 | timestamp |
| WIFI_RETENTION_DAYS | 90 | timestamp |
| AUDIT_RETENTION_DAYS | 180 | created_at |
| RESOLVED_ALERT_RETENTION_DAYS | 180 | resolved_at |

`HISTORY_RETENTION_ENABLED=false` es el valor inicial; cada período admite 0 para
omitir esa tarea, o 1–3650 días. Se elimina lo anterior al inicio del día UTC menos
el período, con el mismo MAINTENANCE_BATCH_SIZE de las métricas. No se archivan
estos registros: habilitar la política implica aceptar su eliminación definitiva.
Las alertas abiertas y las resueltas sin resolved_at se conservan. No se limpian
security_events, processed_events ni inventario en este bloque.

Cada tarea tiene su propia transacción y comparte el advisory lock del compactador.
Un error revierte su lote sin detener las demás tareas; --once devuelve error si
alguna falla. Fallos dentro del bloque de borrado se guardan en maintenance_job_status;
un fallo de conexión, esquema o transacción exterior solo puede registrarse en logs.
Si otra ejecución tiene el lock, se omite el intento sin registrar un éxito nuevo.

GET /api/maintenance/status requiere JWT de administrador. Devuelve tamaño total
(incluye índices y todas las tablas), último éxito de compactación y resultados
históricos. El backlog consulta como máximo lote+1 filas: es un límite inferior
si backlog_truncated=true, no un recuento completo. La UI muestra la fecha de éxito;
un registro anterior no prueba que la política siga habilitada ni que esté saludable.

Antes de activar: probar migraciones e índices con datos grandes y comprimidos,
rollback, límites UTC, alertas reabiertas, bloqueo y batches concurrentes. Revisar
la política Timescale antigua de activity_logs: sus jobs no comparten nuestro lock
y pueden eliminar datos independientemente del nuevo interruptor. No se modificó
esa política automáticamente. DELETE libera espacio reutilizable, no garantiza
reducir archivos en disco. El archivo Parquet continúa pendiente de un caso de uso.

## Sesiones de personas — candidato 2026-11-02

La migración 0017 agrega `session_activity_daily` y la vista `session_activity_combined`. El trabajo `sessions` compacta el detalle en lotes atómicos antes de eliminarlo: conserva fechas, cantidad de eventos, calidad y contadores por persona/sesión/día UTC. Retención de resumen: días de detalle configurados + 365, con eliminación también acotada por lote. Detalle predeterminado 30 días → resumen hasta 395 días desde la observación. No se activó el mantenimiento del piloto. Aprobar esta conservación y validar restauración pertenece al bloque 5. El resumen no conserva títulos de ventanas ni sustituye una copia de respaldo.
