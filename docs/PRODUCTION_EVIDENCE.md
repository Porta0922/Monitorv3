# Evidencias de cierre — 2026-11-21

## Fuente y alcance

Commit publicado: `6a1ef7a61e3f00f6b14a44588d2dd4a96c5a915b`.
Las comprobaciones siguientes no acreditan aceptación productiva de un servidor real.
Documentación conciliada localmente el 21/11; el código candidato ya está fusionado.

| Evidencia | Resultado | Límite |
|---|---|---|
| [Ciclo de vida](https://github.com/Porta0922/Monitorv3/actions/runs/44897152360) | Aprobado | No escritorios Linux nativos/RDP completos |
| [Preflight](https://github.com/Porta0922/Monitorv3/actions/runs/44897152443) | Aprobado | Incluye navegador y fixtures de backups; no destino productivo |
| [Retención](https://github.com/Porta0922/Monitorv3/actions/runs/44897152384) | Aprobado | Datos sintéticos |
| [Stack y recuperación](https://github.com/Porta0922/Monitorv3/actions/runs/44897152420) | Aprobado | Integración aislada, no carga sostenida |
| [Construcción/publicación](https://github.com/Porta0922/Monitorv3/actions/runs/44897994412) | Aprobado | Candidato sin firma |
| [Release 0.2.3](https://github.com/Porta0922/Monitorv3/releases/tag/agent-v0.2.3) | 8 assets publicados | MSI, ZIP Windows, 3 DEB, código, guía y hashes |

SHA256 MSI publicado:
`82e56c1b1cc259f3a75740da51319a648b934f3a9728d04da808a0b0422fe444`.
No se reemplazó un MSI ya publicado conservando versión.

## Piloto Windows: recuperación y actualización

Al comenzar la revisión, servicio y sesión 0.2.2 activos, Docker saludable,
publicador desconectado y más de 2.100 eventos retenidos. Los logs mostraban
`IO error: connection aborted`; el endpoint era loopback:5673.

Se reinició el servicio sin recuperar conexión. Después se reinició únicamente
RabbitMQ, conservando datos. Una conexión AMQP de diagnóstico recibió respuesta.
Se ejecutó la tarea instalada de actualización, que descargó e instaló la release
0.2.3. No se instalaron manualmente los bytes de un MSI local.

Recibo privado: `.local-test/release-0.2.3/automatic-update-acceptance.json`,
21/11/2026 11:22:03 UTC (08:22:03 America/Asuncion):

- `success=true`, versión `0.2.3`, resultado de tarea `0`.
- Hash de configuración idéntico antes/después.
- Servicio `Running`, una sesión de escritorio y `publisher_available=true`.
- Cola descendió a 1 evento durante la comprobación posterior.

La tarea ejecuta como SYSTEM y oculta; el helper administrativo que inició la
comprobación es independiente del mecanismo automático. No se inspeccionó
visualmente el color del icono; su estado proviene de la conexión autenticada.
No se aisló causa raíz: no atribuir la recuperación únicamente a la nueva versión.
Confirmación del broker no acredita por sí sola persistencia/visualización final.

## Evidencia anterior conservada

- Reparación Windows: `.local-test/acceptance-20261120/windows-repair.json` y
  `windows-repair-session.json`; instalación 0.2.2 en
  `.local-test/release-0.2.2/installed-acceptance.json`.
- DEB: registros por distribución en `.local-test/acceptance-20261120/`;
  ejecución en contenedores con systemd simulado.
- Imágenes/SBOM: rutas, identidad de imágenes y hallazgos en
  [IMAGE_SECURITY.md](IMAGE_SECURITY.md).
- Backups/migración: [PRODUCTION_RECOVERY.md](PRODUCTION_RECOVERY.md),
  [ENCRYPTED_BACKUPS.md](ENCRYPTED_BACKUPS.md),
  [RABBITMQ_MIGRATION.md](RABBITMQ_MIGRATION.md).

No publicar `.local-test`, configuraciones ni credenciales. Los informes anteriores
son evidencia acotada a sus artefactos/fechas, no pruebas nuevas de la release.
Pendientes únicos y decisiones vigentes: [TODO.md](TODO.md).

## Recepción posterior en dashboard

Diagnóstico autenticado de API el 21/11 a las 11:24 UTC: 133 registros de aplicaciones,
9 vigentes y observación más reciente 11:23:11 UTC. Cola de 1–2 eventos con captura
activa. Esto acredita recepción reciente, no conciliación exhaustiva de todos los
eventos acumulados. Evidencia privada: `.local-test/release-0.2.3/window-recovery.json`.

## Recurrencia observada al terminar la limpieza

La consulta posterior devolvió versión 0.2.3, `publisher_available=false` y
1.016 eventos pendientes; los cinco contenedores seguían activos y los servicios
con healthcheck saludables. La recuperación inicial no acredita estabilidad.
No se determinó causalidad con la limpieza; no había procesos ni montajes del
piloto usando las carpetas retiradas. Investigación reabierta en TODO.

## Supervisor 0.2.4 instalado localmente

- `cargo check --locked -p activity-monitor-agent` y build release aprobados.
- Regresión contra broker temporal: cierre de canal/conexión, recuperación
  concurrente única, reutilización de conexión sana y publicación confirmada aprobados.
- MSI SHA256: `a483290202345f1717f99ea563b5e96360aaf24b792f04923297165aaa4ddfc5`.
- Recibo `.local-test/release-0.2.4/installed-acceptance.json`: configuración
  preservada, servicio Running y una sesión; en ese instante seguía desconectado.
- Un cliente AMQP independiente recibía cero bytes desde localhost:5673, servido
  por com.docker.backend. Se recreó únicamente RabbitMQ con el volumen existente.
  Después respondió con 525 bytes; el agente reconectó sin reiniciarlo.
- `.local-test/release-0.2.4/recovery-acceptance.json`: 21/11 19:44:05 UTC,
  versión 0.2.4, conectado y 1 evento pendiente (antes más de 1.100).
- API del dashboard a las 19:44:07 UTC: 18 aplicaciones vigentes y última
  observación 19:43:23 UTC. No equivale a conciliación exhaustiva de toda la cola.

El supervisor reintenta la conexión, no repara Docker. Queda observar la recurrencia
del puerto publicado; no se aisló el desencadenante del fallo en Docker Desktop.
Código y MSI nuevos locales, aún no publicados. Preservar el MSI entregado: reutilizar
esos bytes al publicar 0.2.4 o versionar cualquier reconstrucción posterior.

## 2026-11-22 — Consumo del agente

Medición corregida de unos cuatro minutos, con cola real de 2.739 eventos al inicio
y parada/arranque de broker. CPU media estable 0,0842% total; reenvío 0,6217%.
Pico de RAM combinado 171,73 MiB con dos auxiliares; supera referencia provisional
de 150 MiB. Conectado al terminar. Método, datos, defecto del primer contador y
pendientes en [AGENT_PERFORMANCE.md](AGENT_PERFORMANCE.md). No equivale a aprobar
rendimiento prolongado ni ausencia de ralentización.
