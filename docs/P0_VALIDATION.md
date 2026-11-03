# Validación P0 — 2026-10-24

## Cierre de implementación

El alcance de código P0 está cerrado. La comprobación con volumen real y la prueba
específica de DNS Nginx tras recrear el servidor permanecen en P4 por decisión del
usuario. No se ejecutaron para este cierre. Los resultados siguientes son históricos,
no una validación de los cambios P1/P2/P3 posteriores ni una certificación de producción.

## Resultados locales históricos

| Comprobación | Resultado |
| --- | --- |
| Suite del servidor | 7 pruebas aprobadas, incluidas autorización y migraciones |
| Actualización TimescaleDB | 100.000 registros comprimidos; recuento y suma conservados |
| Bloqueo de migración | Espera limitada, rollback y reintento correcto |
| Arranques simultáneos | Una sola versión aplicada |
| Duración de migración | ~0,5 s vacía; ~10 s con el fixture comprimido, en esta máquina |
| Caché y publicador del agente en Windows | 2 pruebas aprobadas; ACK confirmado y rechazo de mensajes sin ruta |
| Transporte nativo Windows → RabbitMQ → API | Una muestra de CPU=42,5 consultada correctamente |
| Retención | 4 pruebas aprobadas |
| Dashboard | Compilación TypeScript/Vite local y Docker correcta |
| Dependencias frontend | npm audit: 0 vulnerabilidades en esta revisión |

## Reproducir

El workflow `.github/workflows/stack-smoke.yml` construye la imagen `validation`,
aplica migraciones a bases temporales, prueba la API y el transporte, y ejecuta
`tests/p0_recovery.py`. Este último solo acepta los contenedores etiquetados con
el proyecto Compose `monitor-refactor-test`: termina el servidor con SIGKILL,
comprueba su reinicio automático, detiene el broker 90 segundos y comprueba
que las muestras persistidas coinciden con las generadas por el probe.

En Windows se puede ejecutar:

```powershell
cargo build --locked --release -p agent-transport-probe
python tests/native_transport.py target/release/agent-transport-probe.exe
```

El probe reutiliza el código del publicador y la caché del agente mediante
referencias de módulo; no instala servicios ni activa recolectores.
La prueba no equivale al ciclo de instalación y actualización del agente completo.

## Límites

No hay copia del volumen real disponible (confirmado por el usuario). La prueba
con datos sintéticos comprimidos no certifica todas las variantes de un volumen
real ni su duración. Esa comprobación externa queda pendiente.

P0 mantiene las tablas nuevas como PostgreSQL normal. La conversión optativa y
las claves están diseñadas en `HYPERTABLE_DESIGN.md`; habilitar compresión de
métricas requiere validar el mantenimiento sobre esos chunks en P1.
La deduplicación y las escrituras del consumidor todavía deben compartir una
transacción para garantizar recuperación ante fallos en medio de un commit.
El workflow se preparó, pero no se ejecutó en GitHub.

Registros locales: `p0-migration-validation.log`,
`p0-windows-transport-tests.log`, `p0-stack-validation.log` y `p0-recovery.log`.

## Actualización de RabbitMQ existente

El hostname del broker es estable (`RABBITMQ_HOSTNAME`, por defecto `rabbitmq`).
RabbitMQ asocia su almacenamiento al nombre del nodo. Antes de actualizar una
instalación anterior con mensajes pendientes, obtener el hostname original con
`docker inspect --format '{{.Config.Hostname}}' <contenedor-rabbitmq>` y asignarlo
a `RABBITMQ_HOSTNAME` en `.env`. No cambiarlo entre recreaciones. Respaldar el
volumen antes de actualizar. El valor por defecto es para instalaciones nuevas;
no migra automáticamente datos asociados a otro nombre de nodo.

En el stack sintético se confirmó que las siete colas estaban vacías antes de
adoptar el nombre estable. Ningún volumen se eliminó durante esa actualización.
