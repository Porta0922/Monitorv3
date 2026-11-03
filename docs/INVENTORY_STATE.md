# Inventario actual y cambios — 2026-10-27

Implementado en código; migración 0005, ejecución real y UI pendientes de P4.

## Cobertura explícita

- linux-dpkg: paquetes con estado installed obtenidos de dpkg-query. Comando con
  timeout de 30 s; salida inválida o fallo produce escaneo incompleto.
- windows-machine-64 y windows-machine-32: claves HKLM Uninstall de cada vista.
  Clave de origen estable, DisplayName y DisplayVersion. Un error de enumeración
  o lectura invalida la fuente entera; no se infieren bajas de un resultado parcial.
- No se incluyen Snap/Flatpak, portables ni HKCU/sesiones de otros usuarios.
  Tampoco se interpreta un nombre de paquete como ruta o hash de ejecutable.

La tarea emite inventory_snapshot, schema_version=2, por monitoring.inventory.
El payload contiene scope, complete, error opcional, interval_days y apps con
source_key/app_name/version. Una fuente completa y vacía sí puede eliminar sus
entradas anteriores. Una fuente incompleta nunca las elimina ni altera.

Nuevo intervalo predeterminado: un día. Configuraciones explícitas existentes
se respetan; no se reescriben snapshots locales de política. Fallos de fuente o
rechazo de la cola local se reintentan con nuevo escaneo en cinco minutos. Todo
reenvío del evento ya aceptado corresponde a la outbox, conservando su event_id.
Se envía la observación completa por fuente para poder reconciliar tras reinicios;
no se almacena una copia completa adicional por cada escaneo en PostgreSQL.

## Persistencia y orden

inventory_current conserva una fila por equipo/fuente/clave de origen.
inventory_scan_state conserva último intento, último escaneo completo, error,
conteo, intervalo esperado y hash canónico del estado confirmado.
inventory_changes registra exclusivamente alta/cambio/baja con antes/después.
La primera observación completa crea altas de línea base, no fechas de instalación.
Un cambio de clave puede aparecer como baja+alta, no se fusiona por nombre.

Reconciliación bajo la transacción de deduplicación/ACK. Bloqueo por fuente y fecha
estrictamente creciente: reenvíos antiguos no reemplazan la última observación.
Hash estable independientemente del orden de apps; un escaneo idéntico solo
actualiza la fecha de confirmación, sin duplicar filas ni generar cambios.
Bajas solo tras comparar una fuente completa, sin afectar otras fuentes.
Máximo 50000 entradas, claves únicas, campos acotados y payload hasta 16 MiB.

La tabla inventory anterior permanece como detecciones históricas. No se migra
su contenido a instalado actual porque no acredita vigencia ni fuente completa.
Las rutas antiguas siguen disponibles; las nuevas fuentes no alimentan esa tabla.

## API e interfaz

GET /api/inventory/current/:device_id y /api/inventory/changes/:device_id, con JWT.
Parámetros: q (subcadena literal, máximo 256 bytes), limit 1–200, offset 0–1000000.
Respuesta items/total/limit/offset/sources. Errores de consulta retornan 500;
equipo inexistente 404. Los conteos/páginas pueden variar entre consultas concurrentes.

La página Inventario permite seleccionar equipo y consultar estado/cambios,
con búsqueda y paginación en servidor. Muestra cobertura, error, último escaneo
completo y fuentes atrasadas (intervalo esperado + un día). El detalle del equipo
conserva aparte las detecciones heredadas, identificadas como no autoritativas.

## Mantenimiento

INVENTORY_CHANGE_RETENTION_DAYS=90: elimina cambios antiguos por lotes cuando
HISTORY_RETENTION_ENABLED=true. 0 desactiva esta tarea; no elimina estado actual
ni estado de fuente. La bandera global sigue desactivada por defecto. Índice por
fecha/id, mismo bloqueo de mantenimiento y registro de resultado/backlog.
No se implementó archivo externo ni purga automática de inventario heredado.

El panel de mantenimiento ahora muestra hasta 200 relaciones físicas de public
y _timescaledb_internal (incluidos chunks), tamaños de tabla/índices y estimación
de filas. Listado alfabético con indicador de truncamiento, no ranking global ni
suma por hypertable. Los totales físicos incluyen TOAST/índices; no sumar raíces y
chunks para deducir el total de una hypertable sin resolver su relación.

## Pendiente

P4: migraciones, reconciliación/replay/concurrencia, fallos parciales de fuentes,
retención, revisión visual y compilación/ejecución Linux del nuevo adaptador.
P1 siguiente: ventana de reenvío y processed_events, conservación de seguridad,
operación/alertas de DLQ. Coberturas adicionales de inventario requieren fuentes
separadas con garantías explícitas de completitud.
