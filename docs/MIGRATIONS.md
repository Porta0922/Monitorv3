# Migraciones del servidor

El servidor aplica `server/migrations/` mediante SQLx antes de abrir la API o
consumir RabbitMQ. Los SQL se incluyen en el binario. `_sqlx_migrations` registra
la versión y checksum; el bloqueo de SQLx serializa arranques simultáneos y cada
migración se ejecuta en una transacción. Un error impide iniciar el servidor.
No editar una migración aplicada: agregar otro archivo numerado.

Docker Compose ya no monta los SQL históricos en `docker-entrypoint-initdb.d`.
La misma migración del servidor atiende instalaciones nuevas y volúmenes
existentes. Los archivos de `migrations/` en la raíz se conservan como referencia
y fixture de compatibilidad; no deben ejecutarse en instalaciones nuevas.

La primera migración conserva las tablas y datos existentes y reconcilia columnas
que faltaban en el esquema antiguo: `activity_logs.created_at`,
`security_alerts.timestamp` y `process_termination_attempts.auto_restarted`.
También normaliza `security_alerts.id` a BIGINT para el modelo Rust.
Las tablas históricas que no usa el servidor no se eliminan ni se trasladan.

Las instalaciones nuevas usan tablas PostgreSQL normales. Las hypertables y
políticas Timescale existentes se conservan. Se retiraron las conversiones
automáticas que fallaban silenciosamente: algunas claves únicas no incluían la
columna de partición y algunas consultas apuntaban a tablas/columnas inexistentes.
Una conversión futura necesita su propia migración y pruebas con datos y chunks
comprimidos; esta primera migración no convierte tablas. El backfill puede
reescribir filas comprimidas: requiere ventana de mantenimiento y espacio libre.
Los cambios de tipo UUID se omiten si la columna ya tiene ese tipo, para evitar
reescrituras innecesarias y conservar las claves foráneas existentes.
Cada sentencia tiene un límite de 120 s y la espera de bloqueos, de 5 s.

## Prueba de compatibilidad

La prueba Rust ignorada por defecto
`postgres_db::migration_tests::fresh_and_legacy_migrations_preserve_data`
requiere un TimescaleDB temporal y permisos para crear/eliminar bases de prueba.
Configurar `TEST_DATABASE_URL` con la base administrativa temporal y
`LEGACY_SCHEMA_DIR` con el directorio raíz `migrations/`. Ejecutar:

```sh
cargo test --locked --release -p activity-monitor-server -- --include-ignored
```

Crea bases con nombres aleatorios, comprueba instalación vacía y esquema antiguo
con 100.000 muestras de actividad comprimidas, prueba el rechazo ante bloqueo,
inicia dos conexiones simultáneamente y verifica una sola migración aplicada,
recuento y suma conservados. Imprime la duración de la migración. Elimina las bases al
completar correctamente; ante un fallo, quedan en la instancia temporal para
inspección. No ejecutar contra el servidor de producción.
