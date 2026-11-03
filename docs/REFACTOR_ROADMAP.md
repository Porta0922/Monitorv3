# Refactorización Monitorv4

## Arquitectura acordada

Agente nativo Windows/Ubuntu/Debian → RabbitMQ → servidor Rust/Axum → PostgreSQL/TimescaleDB.
Dashboard React servido junto al backend. Backend, dashboard, RabbitMQ, base de datos y mantenimiento permanecen en Docker Compose.
RabbitMQ se conserva. No se requiere Docker en las computadoras monitoreadas.

Directorio de trabajo: `C:\Dev\Monitorv32`. Rama local: `refactor/platform-foundation`.

## Orden de trabajo actualizado — 2026-10-24

P0: base de despliegue. P1: datos y entrega. P2: agente multiplataforma y ciclo de vida.
P3: dashboard. **P4: todas las pruebas, compilaciones y validación de entrega**,
por indicación del usuario. El detalle vigente está en TODO.md; los cambios nuevos
no se consideran validados. P2 comenzó por modos de ejecución, persistencia y
plantillas de servicio (AGENT_LIFECYCLE.md). La separación inicial y el IPC local están
escritos en SESSION_COMPONENT.md; enrolamiento, multiusuario y MSI/DEB siguen pendientes.

## Ruta de implementación y criterios de salida

1. **Almacenamiento y despliegue (iniciado).** Compactación transaccional de métricas,
   retención configurable, reinicio de contenedores y pruebas de pérdida/duplicación.
   Unificar después las migraciones SQL y el esquema creado desde Rust, verificando
   instalaciones nuevas y volúmenes existentes. No convertir tablas con claves
   incompatibles a hypertables ignorando errores.
2. **Contrato de telemetría.** Separar recursos del equipo, sesión, inventario y salud
   del agente. Unidades, capacidades y calidad explícitas; identificador de evento
   y versión de esquema. Confirmación de publicación, ACK después del commit,
   deduplicación transaccional, reintentos y cola de errores con límites en RabbitMQ.
   La ventana de deduplicación debe cubrir el máximo de reenvío offline y del broker.
3. **Agente multiplataforma.** Núcleo común más adaptadores Windows/Linux. Servicio
   del sistema y componente de sesión independientes, IPC con permisos restringidos.
   CPU, memoria, disco, red, uptime, inventario y métricas del propio agente.
   Distinguir Linux sin escritorio, X11 y Wayland; no inventar valores no disponibles.
4. **Instaladores y actualizaciones.** MSI/DEB, inscripción del dispositivo,
   servicio Windows/systemd, identidad estable, actualizaciones firmadas y rollback.
   Pruebas instalar → reiniciar → terminar proceso → recuperar → actualizar → desinstalar.
5. **Dashboard y API.** Estado de flota, tabla filtrable, problemas por severidad,
   detalle temporal y gestión de versiones. Unir detalle reciente con resúmenes
   históricos sin solaparlos. Errores de consulta nunca equivalen a cero alertas.
6. **Piloto y entrega.** Pruebas en Windows, Ubuntu y Debian, con y sin sesión,
   desconexiones, recuperación de cola, carga y backup/restauración. Releases
   reproducibles y documentación de instalación, actualización y desinstalación.

## Política objetivo

| Información | Conservación propuesta | Estado |
|---|---|---|
| Recursos y actividad por intervalo | 14 días completos UTC | Implementado en primer incremento |
| Resúmenes diarios de recursos/actividad | 365 días | Implementado en primer incremento |
| Aplicaciones/USB/WiFi detallados | 30 días, con resúmenes donde aporten valor | Pendiente |
| Seguridad y auditoría | 90 días; alertas abiertas sin vencimiento automático | Pendiente |
| Inventario | Estado actual + cambios, evitando snapshots redundantes | Pendiente |
| Identificadores de deduplicación | Ventana mayor al máximo de reenvío | Pendiente de contrato de reenvío |
| Archivo externo opcional | Parquet comprimido, particionado por fecha | Pendiente; desactivado |

No archivar automáticamente todo: aumenta coste y conserva ruido. Exportar solo
información con un uso definido. Un archivo debe tener manifiesto, checksum,
confirmación de escritura y prueba de lectura antes de borrar su fuente.
El archivo histórico no reemplaza los backups de recuperación.

## Primer incremento: alcance exacto

Ver [DATA_RETENTION.md](DATA_RETENTION.md). Solo se compactan
`node_resource_metrics` e `input_activity_metrics`. No se cambia el protocolo
RabbitMQ ni se eliminan dispositivos, inventario, alertas o auditoría.
No se implementaron aún MSI/DEB, el dashboard nuevo, archivo externo ni
compresión Timescale. Las políticas antiguas de las migraciones permanecen
intactas y se deben inventariar al unificar el esquema.
