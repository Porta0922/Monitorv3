# Bloque 4 — cierre funcional del candidato

Fecha: 2026-11-02. Código candidato, no desplegado sobre el piloto. Agente fuente 0.1.0; instalado 3.4.0 intacto.

## Contratos y decisiones de alcance

- **Tiempo de personas:** resumen global, detalle de equipo, métricas diarias y CSV de sesiones leen `session_activity_combined`. La vista reúne detalle y resumen disjuntos, por día UTC, principal y sesión. Los contadores disponibles suman actividad/inactividad, teclas, movimientos y clics; sin captura se conservan null y contadores de calidad. Nunca se interpretan como horas encendido. Son muestras por intervalos, no una reconstrucción segundo a segundo.
- **Legado:** se conservan tablas históricas de entrada de máquina; no se mezclan con horas-persona. El nuevo resumen puede carecer de datos si el agente anterior solo produjo observaciones heredadas. Los recursos de CPU/RAM mantienen sus agregados ponderados propios.
- **Compactación:** migración 0017 y mantenimiento de sesiones transfieren lotes de detalle al resumen dentro de una transacción. Reintentos y datos tardíos no duplican las filas movidas. El mismo bloqueo coordina mantenimiento y preservación de investigaciones. Resumen de sesiones retenido hasta `SESSION_EVENT_RETENTION_DAYS + 365` días; con 30 días de detalle, 395 días desde el día observado. Este valor es una propuesta técnica, cuya aprobación y backups siguen en bloque 5. El trabajo solo corre si se habilita mantenimiento histórico.
- **Calendario:** los días del dashboard y CSV son UTC, incluidos filtros históricos; las fechas iniciales también. Instantes individuales pueden presentarse en hora del navegador con fecha. No se ofrece selección de zona IANA ni días civiles con DST en este piloto; no se aplica el offset actual a un día histórico. Los filtros HTTP inválidos se rechazan y los errores de consulta no se disfrazan de datos vacíos.
- **Ventanas:** último snapshot por principal/sesión, con fecha y vigencia. El agente repite el snapshot aunque no cambie y envía vacío al cerrar ordenadamente. Si el servicio no responde se intenta conservarlo en caché. Ante caída abrupta se usa antigüedad; no se inventa un instante exacto de cierre ni se confirma una ventana actualmente abierta. Los snapshots vacíos sustituyen anteriores y los atrasados no los restauran.
- **Inventario y eventos:** inventario completo reconcilia altas/cambios/bajas; escaneos incompletos y atrasados conservan el estado conocido. Deduplicación transaccional por evento para USB/WiFi y demás telemetría. USB conserva el instante observado en la retransmisión. Una alerta recurrente crea una ocurrencia nueva; no reescribe la resolución de la anterior. Resolver una ya resuelta no vuelve a cambiar notas/fecha. Las investigaciones conservan sus intervalos de evidencia.
- **Política:** snapshot completo por equipo, o herencia global si no hay override; sin combinación parcial por campo. Revisión optimista con 409 al guardar sobre una revisión antigua. Listas duplicadas o contradictorias se rechazan tanto en servidor como en agente. Pausa/reanudación, restauración de defaults y persistencia offline comprobadas; salud reporta huella aplicada, distinta de una mera orden guardada.
- **Comandos:** solo `ping`, `status`, `run_diagnostic`, sin payload configurable. Consultas de lectura, sin ejecutar comandos del sistema ni desinstalar/actualizar/reconectar remotamente. Reclamo atómico de hasta 25, reentrega tras 60 segundos sin ACK y vencimiento de una hora. ACK repetido idéntico aceptado; contradictorio o vencido rechazado. Reejecución de lectura tras reinicio es posible: no se promete exactamente una ejecución. Agente rechaza acciones fuera de lista y órdenes sin vencimiento válido. El nuevo agente exige servidor actualizado.
- **Exclusiones visibles:** mapas de calor por coordenadas fuera de 0.1; captura global de entrada/ventanas Wayland no certificada. Recursos e inventario no implican disponibilidad de captura de escritorio. X11/Windows necesitan aceptación nativa del bloque 6.

## Evidencia focalizada

- Servidor compilado: 17 pruebas aprobadas y 3 ignoradas que requieren entornos externos específicos (no contabilizadas como ejecutadas). Agente comprobado y dashboard TypeScript/Vite construido.
- PostgreSQL/Timescale desechable: diecisiete migraciones, dos escritorios por principal, snapshots vacíos/atrasados, contadores null, totales de todas las páginas y compactación por lotes/reintentos conservando valores.
- Siete casos PostgreSQL WASM: compactación, rollback, límites UTC, medias ponderadas, datos tardíos y exclusión de contadores heredados del resumen de personas. No emula Timescale.
- Roles productivos y diez trabajos de mantenimiento verificados sobre una DB vacía aislada; permisos del nuevo resumen incluidos.
- `tests/identity-stack.py --block4`: servidor productivo, DB y RabbitMQ TLS sintéticos, identidades reales, consultas/roles, comandos y ACK, políticas/global/equipo/revisión, resolución y recurrencia de alertas. Publicador nativo contra consumidor real: inventario completo/parcial/atrasado, cambios esperados y duplicados USB/WiFi rechazados; fecha USB preservada.
- Tres pruebas nativas Windows de políticas: persistencia/reapertura, defaults, rechazo de configuración inválida y arranque/pausa/reanudación de recolectores. El reemplazo atómico requirió ejecución fuera del sandbox, solo sobre directorios temporales de ensayo.

Esto cierra implementación y validación focalizada del bloque 4 para el alcance indicado. No acredita todavía aceptación de interfaz en navegador, apagados reales, instaladores, Linux nativo, estrés ni producción. Las pruebas históricas de migración de una DB antigua siguen pendientes por falta de una copia representativa.

## Puerta de entrada al nuevo piloto

1. Respaldar el volumen local y la configuración privada; conservar el instalador anterior y probar el procedimiento de reversión.
2. Construir imágenes candidatas y el paquete 0.1.0 correspondiente a este código. Comprobar versión y hashes; preparar la transición desde MSI 3.4.0 sin forzar un downgrade que pierda identidad, claves o pendientes.
3. Migrar una copia local de la DB, iniciar servidor/broker/dashboard candidatos y verificar login, recepción, métricas, snapshots y sesiones en navegador.
4. Entregar el instalador del agente para la prueba controlada en el equipo del usuario; verificar reinicio y reconexión. No sustituir el piloto actual hasta superar los pasos anteriores.

El piloto local puede prepararse ahora. No requiere que estén terminados firma pública, certificación multiplataforma o autorización productiva. Su fecha de instalación depende de completar paquete/transición y respaldo; no se anuncia una fecha ni un instalador listo antes de tenerlos.
