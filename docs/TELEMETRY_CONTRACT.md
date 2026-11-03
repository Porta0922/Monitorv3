# Métricas actuales, capacidades y salud — 2026-10-27

Implementado en código; migración/despliegue y pruebas integrales pendientes de P4.

## Agente

ResourceMonitor usa sysinfo 0.29 en Windows/Linux y agrega system_metrics con
schema_version=1 al input_summary de la ruta monitoring.heartbeat. La sesión no
produce recursos del sistema. La muestra inicial pertenece a la misma tarea
supervisada, sin un segundo productor independiente.

- CPU en porcentaje. Primera lectura null/warming_up: requiere dos observaciones.
- RAM en bytes y porcentaje; legado memory_used_mb expresado en MiB. Se corrigió
  una división entre 1024 cuando sysinfo ya devuelve bytes.
- Disco por volumen: montaje, capacidad, espacio disponible y porcentaje usado.
- Red por interfaz: contadores de bytes y tasas medias en bytes/s. Primera lectura,
  interfaz nueva o contador reiniciado produce tasa null; no inventa cero.
- Uptime del sistema en segundos; separado del uptime del proceso agente.
- Inventario básico del sistema: CPU lógicas/modelo, versión de SO y kernel.
- Capacidades: rol, SO, arquitectura, contexto de escritorio y entrada disponible.

Máximo 128 discos/interfaces por muestra. No se suman interfaces (virtuales y
físicas pueden contar el mismo tráfico). Ausencia de entradas es no disponible,
no evidencia de que el equipo carezca de discos/red. sysinfo no informa un error
de permisos granular para todos los getters: no se afirma disponibilidad plena.

La salud se publica por monitoring.health cada 300 s. Incluye tareas, reinicios,
último error, política pausada, capacidades y cola pendiente. Si no puede leerse
la cola, el tamaño es null y la salud se degrada. Diagnóstico por recolector:
supervised, disabled, unsupported_context, stopped, not_started. Supervisado no
certifica éxito de cada captura; diagnósticos específicos de cada adaptador siguen
pendientes. Salud de sesiones múltiples todavía no está consolidada.

## Servidor y almacenamiento

monitoring.health se vincula a heartbeat_queue, usando el mismo flujo de
transacción, deduplicación, reintentos y DLQ. Antes no tenía consumidor allí.
El servidor valida tamaño, tipos, límites y timestamp de las nuevas muestras.

Migración 0004: device_telemetry_state contiene como máximo dos filas por equipo:
resources y health. Actualización solo si observed_at es posterior; un replay
antiguo no reemplaza el estado reciente. Se conserva received_at para diagnóstico.
Este bloque no crea historial de discos/red/diagnóstico ni requiere purgar snapshots.

CPU/RAM siguen entrando al historial existente solo cuando las tres medidas
están presentes; no se rellenan mediciones ausentes con cero. Los registros
históricos ya creados no se corrigen automáticamente: faltan metadatos fiables
para distinguir unidades/ausencia de versiones antiguas. La primera lectura de
CPU no entra en ese historial. Inventario de software por cambios sigue en P1.

## API y dashboard

GET /api/devices/:id/telemetry (también alias sin /api) requiere JWT de administrador.
Devuelve resources/health nulos si todavía no hay datos del nuevo contrato;
404 para equipo inexistente y 500 ante error de consulta. Cada muestra contiene
observed_at, received_at, stale y data. Recursos atrasados después de 180 s;
salud después de 900 s. Son umbrales explícitos, no un diagnóstico de conectividad.

La lista de equipos agrega agent_health y agent_health_error sin consultas N+1.
La flota separa conexión de salud y no deduce degradación de las alertas. El detalle
incluye recursos, hardware, capacidades, fecha y diagnósticos por recolector/tarea.
Muestra actual independiente de la fecha histórica seleccionada; null se presenta
como no disponible, distinto de un valor cero. Un error de refresco conserva la
muestra anterior con aviso. Datos heredados siguen consultables por sus rutas.

## Pendientes

P4: aplicar migración en stack aislado, publicar métricas/salud reales, verificar
orden/replay y errores, revisar UI y matriz OS al completar el bloque de código.
No se ejecutó esa campaña ni se actualizó el stack en esta continuación.
P1: inventario por estado/cambios, retención y operación de DLQ.
P2/P3: errores específicos de captura, salud de sesiones y calidad de las series
históricas de entrada de usuario. No interpretar cero histórico como disponibilidad.
