## 2026-11-03 — Recuperación nativa y preparación de publicación

Diagnóstico de sesiones confirmado con principal, entrada disponible y captura reciente. El vacío inicial correspondía al primer reporte del servicio, publicado antes de recibir la sesión; consolidación periódica cada 300 segundos. No fue necesario modificar el binario 0.1.0.

Prueba acotada del piloto Windows: RabbitMQ detenido 75 segundos, cola máxima de 11 pendientes, restauración del broker y drenaje a cero, identidad conservada. Caída controlada del proceso del servicio: SCM lo recuperó automáticamente en 16 segundos, modo de inicio Auto, recepción posterior de actividad/ventanas/contadores y un solo equipo. Esto no equivale a reiniciar Windows ni a una desconexión prolongada. Evidencia privada: `.local-test/candidate-0.1.0/recovery.json` y `service-recovery.json`.

Cinco scripts de integración adaptados al login de cookies/CSRF; credenciales sintéticas explícitas en el overlay desechable. `tests/smoke_stack.py` aprobado en `monitor-publish-check` (proxy, login, ingestión, duplicados, permisos, métricas diarias y mantenimiento); entorno retirado después. `tests/local_desktop_smoke.py` aprobado contra el piloto real. Los otros scripts adaptados tienen sintaxis comprobada, no se repitió toda su campaña. Dockerfiles de validación Linux incluyen ahora los recursos `agent/ui` requeridos por `include_str!`; builds Linux completos pendientes.

Preparación de GitHub: binario heredado retirado del índice Git, conservado en disco; exclusiones de paquetes/datos/claves; revisión local `scripts/check-publication.py` y workflow equivalente. Release por tags `agent-v*`, versión validada, build `--locked`, SHA256 y publicación como borrador (0.x prerelease). No se hizo commit, push ni reescritura del historial.

Bloqueo real: token fine-grained en historial de `usb/agent-config.json` (fc887c4, 2026-06-24), retirado del árbol actual. Pendiente identificar/revocar en GitHub con el usuario. Credenciales antiguas de RabbitMQ retiradas también de instaladores heredados; rotarlas si el entorno antiguo continúa activo. La revisión actual no sustituye auditoría completa del historial, dependencias o seguridad. Ver [GITHUB_PUBLICATION.md](GITHUB_PUBLICATION.md).

## 2026-11-03 — Piloto limpio 0.1.0 instalado

Por autorización explícita del usuario se descartaron los datos del piloto anterior, sin respaldo ni migración: desinstalación coordinada del MSI 3.4.0, eliminación acotada de carpetas de Monitor del servicio y la sesión, y recreación de los tres volúmenes del proyecto `monitorv32-local`. Los otros proyectos Docker no se modificaron.

Servidor/dashboard reconstruidos; diecisiete migraciones aplicadas sobre DB vacía. MSI privado 0.1.0 compilado e instalado, con servicio automático y sesión de usuario enrolada. Credenciales nuevas en `.local-test/ACCESO-LOCAL.md`, sin publicarlas. PostgreSQL queda interno a Docker porque Windows rechazó el puerto 55433.

Comprobado: login con cookie y código MFA vacío para la cuenta bootstrap local; un equipo con versión 0.1.0; recursos, actividad, snapshots de ventanas recientes y contadores disponibles por sesión; CSV de sesiones y vista local. Broker conectado. La cola transitoria puede contener eventos entre ciclos de envío; no se certifica desconexión prolongada. El texto del login ahora aclara que el código solo corresponde a cuentas con MFA configurado. Build TypeScript/Vite y despliegue del dashboard aprobados.

Este perfil usa desarrollo local HTTP/AMQP en loopback; no valida la adopción productiva de TLS, cuentas MFA e identidades por agente. Instalador privado sin firma, no redistribuible. El binario y SHA256 del MSI están registrados en `.local-test/candidate-0.1.0/local-package.json`; comprobación local en `verification.json`.

Pendiente: reinicio real de Windows y recuperación, desconexión prolongada/reenvío, aceptación UX, Ubuntu/Debian, firma, privacidad/retención, backup/restore y puertas productivas. La transición que conserva datos 3.x→0.1.x no fue ensayada: para este equipo se eligió reinstalación limpia. Las entradas anteriores se conservan como historial.

## 2026-11-02 — Cierre del bloque 4, candidato sin desplegar

Contadores por persona unificados entre resumen global/equipo, histórico y CSV; migración 0017 y compactación transaccional por sesión. Snapshots periódicos y cierre ordenado, antigüedad visible, filtros estrictos y errores HTTP explícitos. Políticas completas con herencia/revisión y validación; comandos de lectura con vencimiento, reentrega y ACK idempotente. Inventario parcial/atrasado, duplicados USB/WiFi y recurrencia de alertas comprobados contra el consumidor real.

Evidencia final: servidor 17 pruebas aprobadas, 3 ignoradas por requerir entornos externos específicos; agente 3 pruebas de políticas aprobadas en Windows; dashboard construido; 17 migraciones y lecturas/compactación en Timescale aislado; 7 casos de retención aprobados; roles de mantenimiento aprobados; `tests/identity-stack.py --block4` aprobado con TLS y broker reales en entorno sintético. Las pruebas ignoradas no se presentan como ejecutadas.

Alcance y límites: [BLOCK4_CLOSURE.md](BLOCK4_CLOSURE.md). Informes UTC; sin selector IANA/DST, mapas de calor ni certificación de captura global Wayland. El cierre corresponde a implementación y comprobación focalizada, no a aceptación nativa o producción. Esta entrada sustituye el estado parcial de las entradas anteriores, conservadas como historial.

Siguiente: respaldo y reversión, imágenes y MSI 0.1.0, transición desde 3.4.0 conservando identidad/claves/outbox, migración de una copia y nuevo piloto local. El instalado y sus datos no fueron sustituidos. Permanecen abiertos bloques 1 y 5–10.

## 2026-11-02 — Bloque 4: ventanas por sesión y contadores diarios

Implementado:

- Migración 0016: último snapshot independiente por equipo/principal/sesión. No mezcla escritorios del mismo usuario. Un snapshot vacío sustituye el anterior y mensajes atrasados no lo restauran. La API incluye origen, principal, sesión y vigencia; reportes de más de 180 segundos o fechas futuras fuera de tolerancia se marcan sin vigencia confirmada.
- Dashboard muestra últimas ventanas reportadas, fecha y atribución; nunca garantiza que sigan abiertas. Los fallos de consulta ahora devuelven HTTP 500 en lugar de una lista vacía. La consulta duplicada en Database delega al repositorio común.
- Consulta única `session_activity.sql` para pantalla y CSV: actividad/inactividad de personas, teclas, movimientos y clics solo cuando la captura está disponible. Totales del día UTC independientes de la paginación; ausencia de muestras permanece null. Muestras disponibles, no disponibles y desconocidas separadas.
- Resumen por equipo incorpora contadores de sesiones con detalle plegable. Exportación diaria exclusiva de operador/admin y auditada, con límite explícito de 10.000 sesiones; si lo supera devuelve 413 sin CSV parcial. No se mezcla tiempo de personas con métricas heredadas de máquina.
- Exportación de actividad rechaza UUID/fechas/offset inválidos, rangos invertidos o mayores de 366 días. La fecha omitida se calcula con el offset enviado. Un UUID inválido ya no implica todos los equipos.

Validación: servidor comprobado con cargo check y dashboard TypeScript/Vite construido; prueba focalizada de exportación aprobada con seis filtros inválidos rechazados antes de acceder a DB; `tests/session-read-models.py` aprobado sobre Timescale/PostgreSQL desechable aplicando las dieciséis migraciones, dos escritorios del mismo principal, snapshots vacíos/fuera de orden, datos antiguos, exclusión de legado, totales con página limitada y ausencia de muestras. No se aplicaron migraciones ni imágenes nuevas al piloto. Docker Desktop se inició para esa comprobación.

El bloque 4 permanece abierto: falta consolidación del resumen global y del histórico compactado por sesión, coherencia temporal completa/DST, contratos de otras rutas, políticas/comandos y demás criterios de TODO. La vista de ventanas expresa observaciones; no incorpora aún un evento explícito de cierre de sesión.

## 2026-10-30 — Bloque 3 cerrado en implementación y verificación focalizada

- Sesiones opacas en DB (migración 0015), cookie HttpOnly/Secure/SameSite en producción, CSRF y restauración sin JWT en localStorage. Expiración de ocho horas y treinta minutos sin peticiones autenticadas.
- MFA TOTP obligatorio para administradores productivos, protección de reutilización, límites de login/API, cuentas Argon2id, generación revocable y restricciones inmediatas de rol. Producción rechaza Bearer humano y bootstrap.
- Pantalla de sesiones propias, revocación administrativa, estado MFA e intentos fallidos. Helper Linux para cuentas, recuperación y emergencia deshabilitada; archivos privados y auditoría local sin secretos.
- Auditoría de operaciones sensibles con actor, UUID cuando corresponde y resultado; exportaciones para operador/admin, escape de fórmulas CSV, CSP y tema inicial externo compatible.

Evidencia: build de servidor y TypeScript/Vite aprobados; dos pruebas TOTP/rol y una CSV aprobadas; recorrido completo de identidad de agentes aprobado durante el bloque. El 2026-10-30, `python tests/identity-stack.py --human-only` terminó con código 0: quince migraciones en DB desechable, cookies productivas, MFA/replay, CSRF, permisos en ambos prefijos, configuración inválida, reducción/promoción, deshabilitación, generación, expiración/inactividad, revocación propia/ajena, proyección de cuentas sin secretos, exportación/investigaciones de operador y auditoría por equipo. Nginx desechable validó HTTPS, CSP y archivo del tema. Helper Linux validó creación/cambios/recuperación, escritura atómica y modos privados; en ese ensayo el hasher fue simulado, mientras que el recorrido HTTP utilizó Argon2id real del servidor.

No se migró ni actualizó el piloto; no se instalaron paquetes ni se publicó el repositorio. La aceptación de navegador y procedimientos con responsables/certificados reales corresponde a la campaña integrada. Fuente del agente continúa 0.1.0 sin publicar; piloto 3.4.0 intacto.

Siguiente: bloque 4, exactitud funcional y coherencia de métricas. Permanecen abiertos 1 y 5–10 según TODO; producción no aprobada.

## 2026-10-29 — Cierre de implementación del bloque 2: identidad y transporte

- Servidor: migración 0014, códigos de enrolamiento de una hora/uso único, reintentos idempotentes, renovación de 30 días con transición máxima de 48 horas, auditoría sin secretos y reconciliación de usuarios del broker.
- RabbitMQ: usuario por credencial, publicación por UUID/tipos permitidos, sin consumo ni configuración para agentes, límites de conexiones/canales/tamaño y colas. Cierre de conexiones al revocar; verificación adicional de identidad en el consumidor y cuarentena de mensajes inválidos.
- Agente: credenciales cifradas con AES-GCM, actualización atómica y bloqueo entre procesos, renovación automática y recuperación mediante código nuevo. Se conservan UUID, clave de caché y pendientes. Producción exige TLS y secretos explícitos. El agente instalado no fue reemplazado; fuente sigue 0.1.0 sin publicar.
- Dashboard: panel administrativo de credenciales en Diagnóstico, código visible solo durante la sesión de la vista, vencimiento/estado y revocación confirmada.
- Operación: generador productivo actualizado, helper de configuración del agente y mantenimiento de certificados con rollback. No se ejecutó una renovación real ni un despliegue candidato.

Comprobaciones: compilación de servidor y publicador nativo aprobada, cargo check del agente aprobado y build de dashboard aprobado. Python: sintaxis de herramientas aprobada. Compose candidato validado sin resolver archivos privados ni iniciar servicios.

`tests/identity-stack.py` aprobado sobre PostgreSQL/RabbitMQ desechables: migraciones reales, TLS/AMQPS, permisos exclusivos de administrador, enrolamiento y repetición segura, rechazo de otro UUID en políticas/comandos/actividad, rechazo de consumo y creación de colas, payload ajeno en topic propio conservado en DLQ sin métricas, publicación del transporte real del agente, renovación/reintento, revocación con conexión abierta y rechazo de CA no confiable. El módulo real de credenciales del agente recupera respuestas perdidas de enrolamiento/renovación y reenrola con un código nuevo conservando UUID y archivo de pendientes de ensayo. No se ejecutaron monitores ni servicios nativos contra datos personales.

Límites: certificados sintéticos, datos sintéticos, servidor/probe nativos en Windows y broker/DB Linux en Docker. No acredita instaladores finales, rotación de CA, emisor real, reinicio del anfitrión Linux ni aceptación del perfil completo. Las herramientas administrativas Linux de despliegue/rollback tienen verificación de sintaxis; su ejecución operativa con certificados reales sigue pendiente. No se tocó el piloto ni se publicó el repositorio.

Siguiente bloque: 3 (acceso humano, sesiones y auditoría). Permanecen abiertos despliegue seguro, exactitud funcional, privacidad/backup, aceptación multiplataforma, paquetes/firmas, UX/accesibilidad, campaña integrada y operación final según TODO.

## 2026-10-29 — Identidad individual HTTP (bloque 2 parcial)

Implementados migración 0013, emisión/listado/revocación exclusivos de administrador, tokens aleatorios con hash en DB, expiración y dos credenciales activas para transición manual. Autorización ligada al UUID solicitado y al propietario de comandos. Auditoría sin tokens. Producción rechaza credenciales compartidas; el piloto no fue actualizado ni migrado.

Verificación acotada: compilación y dos pruebas de identidad/hash, tres de configuración y una de permisos administrativos con ambos prefijos API aprobadas. También pasó la aplicación de las trece migraciones y los trabajos de mantenimiento con roles limitados en una base vacía desechable; no se tocaron datos del piloto. No se acredita todavía ciclo integrado de emisión/revocación con equipos reales. Pendientes: identidad/permisos RabbitMQ, enrolamiento de uso único, renovación/recuperación, UI administrativa y matriz integrada. Documentación: AGENT_IDENTITY.md, README y TODO.

## 2026-10-29 — Tres controles del candidato productivo

- Implementación: roles separados `monitor_owner`/`monitor_app`/`monitor_maintenance`, trabajo de migraciones previo al servidor y comprobación de checksums sin DDL en la aplicación.
- Implementación: targets de producción y UID 65532 para servidor/dashboard/migraciones/mantenimiento; raíz de solo lectura, capacidades retiradas y secretos limitados por servicio.
- Implementación: HTTPS en 127.0.0.1:18443 y AMQPS en 127.0.0.1:25671, certificado/CA externos, generador con validación de cadena/nombres/clave/vigencia y rechazo de AMQP en producción.
- Verificación acotada: tres pruebas unitarias de configuración Rust aprobadas; Compose combinado renderizado y revisado sin iniciar el candidato; `tests/production-roles.py` aprobado con las doce migraciones como owner, límites de privilegios, mantenimiento de métricas y nueve trabajos históricos sobre base vacía desechable; `tests/production-nginx.py` aprobado con Nginx sin root, filesystem de solo lectura, TLS validado y cabeceras, usando certificado temporal de ensayo y la imagen local de dashboard.
- Alcance de la evidencia: no acredita datos reales, chunks comprimidos, imagen nueva construida completa, arranque productivo Linux, AMQPS integrado ni certificados reales. Los contenedores temporales se eliminaron; no se modificó el piloto.
- Siguiente implementación: identidad/enrolamiento/revocación por equipo; después renovación de certificados y cierre de controles restantes del TODO. La validación integrada del candidato requiere infraestructura y certificados preparados.
- README, guía productiva y TODO actualizados. Agente fuente sigue 0.1.0; piloto instalado intacto.

# P4 — Primera tanda de validación

Fecha: 2026-10-24. Cambios locales, sin despliegue ni instalación de servicios.

## Resultados

- `npm run build` en dashboard: aprobado antes y después de corregir useAuth.
  TypeScript y Vite generan el artefacto; esto no valida visualmente la UX.
- `npm run lint`: 28 errores y 1 advertencia. Incluye any explícitos, estado en
  efectos y dependencias/variable sin uso en HeatmapsPage. Resuelto en la segunda tanda.
- useAuth: inicialización diferida desde el almacenamiento de autenticación,
  eliminando el efecto de montaje y su render adicional. Lint individual aprobado;
  lint completo repetido en la segunda tanda; navegación/autenticación aún pendientes.
- `npm test` en tests/storage: 7 aprobadas, 0 fallidas, 0 omitidas. Compactación,
  promedios ponderados, reintentos, datos tardíos, rollback, límites UTC y nulls.
  Motor PGlite: no valida compresión ni políticas TimescaleDB.
- `cargo check -p activity-monitor-agent --locked`: aprobado en Windows.
  Una advertencia: register_windows_restart sin uso. No enlaza un ejecutable ni
  demuestra el comportamiento del servicio o los recolectores.
- ast.parse del enrolador Linux y Parser.ParseFile de enroll-session.ps1,
  register-service.ps1 y start-session.ps1: sintaxis válida; scripts no ejecutados.
- Docker Engine responde, versión 29.7.2. No se recrearon contenedores ni volúmenes.

## Segunda tanda — 2026-10-25

- Lint completo del dashboard aprobado: sin errores ni advertencias.
- Build TypeScript/Vite aprobado después de las correcciones.
- Eliminados any explícitos: errores desconocidos se estrechan antes de leer message;
  respuestas sin contrato tipado permanecen unknown, no se certifica validación runtime.
- Colecciones, historial diario y métricas vinculan resultados a su consulta. Al cambiar
  parámetros se deriva el estado de carga sin resetear estado dentro del efecto;
  respuestas anteriores no actualizan la consulta nueva. Recorrido visual pendiente.
- HeatmapsPage ya no simula una consulta inexistente; informa que no está disponible
  y desactiva el selector hasta conectar la API.
- `cargo test -p activity-monitor-server --locked --bin activity-monitor-server`:
  7 aprobadas, 0 fallidas, 1 omitida (integración de migraciones). Cobertura de JWT,
  contraseñas, rutas/autorización y límites del intervalo diario. Cinco advertencias
  del compilador: imports y campos sin uso; no se ocultaron.
- Stack monitor-refactor-test inspeccionado, sin recreación ni modificación. RabbitMQ
  pasó de unhealthy a healthy por sí solo. Diagnóstico: healthchecks excedieron 10 s
  pese a informar nodo arrancado; posteriormente ambos chequeos completaron. No es
  evidencia suficiente de caída del broker ni validación de recuperación.
- Los contenedores existentes siguen sin representar el código actual.

## Siguiente cobertura

Validar integración del backend, agente en Ubuntu/Debian, IPC/políticas/colas,
permisos efectivos, recuperación y Nginx con servidor recreado. Validación visual
pendiente. Instaladores/rollback aún incompletos en P2; no pueden darse por validados.
La prueba con volumen histórico real sigue pendiente por falta de una copia.

## Tercera tanda — integración actual y UX, 2026-10-25

Servidor y dashboard compilados en Docker y actualizados exclusivamente en
monitor-refactor-test. API healthy después de actualizar; volúmenes conservados.

- Migración sintética actual: 1 integración aprobada, 0 fallidas (40,47 s bajo carga).
  Incluye base vacía, legado con 100.000 filas comprimidas, bloqueo/rollback y
  arranques concurrentes. No sustituye copia de datos reales.
- tests/smoke_stack.py ampliado y aprobado: proxy, login, ingestión AMQP, rutas por
  dispositivo, duplicado, rechazo sin JWT, consulta SQL diaria con CPU 42,5 %, estado
  de mantenimiento y rango invertido rechazado. No prueba deduplicación concurrente.
- tests/nginx_recreation.py aprobado: IP 172.18.0.5 → 172.18.0.8. Misma identidad y
  StartedAt del dashboard; respuesta ready recuperada sin reiniciar Nginx.
  Reserva temporal de IP eliminada y servidor restaurado. Añadido a CI; CI remota no ejecutada.
- Navegador integrado, vista aproximada 614 px: login, menú móvil, búsqueda q en URL,
  estado vacío, limpiar filtros y diálogo con Escape/retorno de foco comprobados.
- Encontrado desbordamiento por sr-only absoluto fuera del contenedor de tabla.
  .table-scroll ahora establece position: relative. Nueva imagen compilada/aplicada;
  documento 604 px dentro de viewport 614 px con tabla cargada (antes 634 px).
- Métricas diarias muestra CPU 42,5 %, RAM 25 %, origen Muestras y actividad 0,01 h.
  Panel de mantenimiento muestra tamaño DB y última compactación. Evidencia local:
  target/p4-metrics.png (artefacto ignorado por Git).
- API parada de forma controlada: aviso "No se pudo consultar: resumen, equipos".
  Después de restaurarla, Actualizar recupera datos. Logout redirige a login.
  Hallazgo menor pendiente: título del documento conserva el de métricas tras logout.

La revisión visual es parcial: faltan escritorio ancho, móvil pequeño, lector de
pantalla, contraste completo y escenarios de administración. El stack queda levantado
con código actual. Falta validación integral del agente, instaladores y recuperación.


## Bloque de entrega y ciclo de vida — 2026-10-26

- Servidor: 7 pruebas ordinarias aprobadas; 3 integraciones ignoradas por defecto.
  Se ejecutaron las 2 nuevas contra PostgreSQL/RabbitMQ reales, con DB y vhost
  temporales: rollback completo ante error SQL, 16 entregas concurrentes, tres
  reintentos y DLQ, reparación/replay con deduplicación, JSON inválido conservado.
- Windows: 9 pruebas focalizadas aprobadas (supervisor, políticas, caché, IPC,
  reemplazo atómico y pausa/reanudación de recolector simulado).
- Debian 12: binario de pruebas compilado y 10 pruebas focalizadas aprobadas,
  incluidas las anteriores y SIGTERM. Log local: target/p2-debian-validation.log.
  Se corrigió reutilización de artefactos desactualizados en el cache Docker
  forzando actualización de timestamps de las fuentes antes de compilar.
- Docker Desktop sufrió errores de motor durante la preparación; tras recuperarlo
  se limitó la compilación a un trabajo. No se atribuye una causa sin diagnóstico.
- Workflow agent-lifecycle preparado para Windows, Ubuntu 22.04/24.04 y Debian;
  no ejecutado remotamente. Los contenedores de validación no instalan servicios
  ni ejecutan recolectores de escritorio.

Ubuntu 24.04: compilación y 10 pruebas focalizadas aprobadas con caché de compilación
independiente de Debian. Log local: target/p2-ubuntu-validation.log.

Pendiente: Ubuntu 22.04, SCM/systemd instalado, cierres nativos, corte de proceso
entre confirmación y ACK, operación de DLQ y despliegue de esta versión del servidor
en el stack aislado. Las pruebas reales de broker ejecutaron el consumidor nuevo
en el proceso de pruebas; el stack Docker no incorpora todavía este bloque.


## Continuación: despliegue y apagado real — 2026-10-26

- Imagen release del servidor reconstruida y desplegada en monitor-refactor-test.
  Smoke completo aprobado: proxy/login, ingestión y deduplicación RabbitMQ, rutas
  parametrizadas, rechazo sin autorización, métricas diarias y mantenimiento.
- Puerto host del dashboard devolvía respuesta vacía aunque Nginx y API funcionaban
  dentro de Docker. Reiniciar únicamente el dashboard restableció el acceso.
- Web local del agente migrada de hilo bloqueante a conexiones Tokio supervisadas.
  Dos pruebas nuevas aprobaron cierre con petición incompleta, liberación del puerto,
  autenticación exacta y límite del cuerpo. Windows y ambos Linux aprobados.
- Hooks/tray Windows con señal cooperativa; prueba de propiedad/cierre de hilo aprobada.
  Windows: 12 pruebas focalizadas únicas aprobadas entre este bloque y el anterior;
  Debian/Ubuntu: 13 únicas cada uno. El filtro lifecycle_tests incluye las nuevas
  pruebas web/nativas; repetir web_lifecycle_tests en CI no suma cobertura distinta.
- tests/agent_runtime.py aprobado en Debian 12 y Ubuntu 24.04: proceso real en modo
  servicio, red deshabilitada, recolectores pausados, volumen Linux independiente.
  SIGTERM produce exit 0 sin reinicio; SIGKILL al agente hijo de init produce un
  reinicio por on-failure. Identidad idéntica; pendientes pasan de 1 a 3 conservando
  el ID original y SQLite integrity_check=ok. Contenedores/volúmenes propios retirados.
- Primer ensayo con SQLite en bind mount Windows falló al reiniciar después de leerla
  desde el host. El ensayo definitivo copia la DB detenida desde volumen Linux;
  no modifica la DB del agente al examinarla. Inyección inicial dentro de PID 1 y
  docker kill no representaban una caída recuperable; se corrigió usando init y
  SIGKILL solo al hijo, sin indicar parada administrativa al motor.

Límites: proceso bajo Docker no prueba systemd ni SCM. La sesión Windows actual
no tiene token de administrador; no se instaló un servicio en el host. Falta
escritorio real (hooks, menús modales, sesión), cancelación rdev Linux, Ubuntu 22.04
y ejecución remota del workflow. Persisten advertencias previas de compilación.


### Ciclo administrativo — 2026-10-27

Implementados scripts locales Linux/Windows de actualización con hash y rollback
ante fallo de arranque, exclusión mutua y desinstalación conservando datos. Retirados
autoactualizador del repositorio antiguo y autodesinstalación por batch destructivo.
Pruebas aprobadas con filesystem real y gestores de servicio simulados: Windows,
Debian y Ubuntu. Detalles y comandos en PLATFORM_LIFECYCLE.md. Pendientes servicios
reales, MSI/DEB, firmas, reparación/reinstalación, sesiones y recuperación ante corte
de energía durante una actualización. Sin cambios publicados en GitHub.

Comprobación final del bloque 2026-10-27: cargo check Windows aprobado; cuatro
pruebas del filtro lifecycle_tests aprobadas. Binario y suite focalizada Debian/Ubuntu
compilados y aprobados tras retirar autoactualización heredada (logs locales
target/p2-debian-oct27.log y target/p2-ubuntu-oct27.log). Proceso real Debian repetido:
exit 0, un reinicio por fallo, identidad conservada, pendientes 1→3 incluyendo
el original. Imagen Ubuntu fijada al digest observado durante esta validación.

### Métricas/capacidades/salud — comprobaciones acotadas 2026-10-27

Cargo check Windows agente/servidor aprobado y TypeScript del dashboard aprobado.
ESLint ejecutado solo sobre archivos frontend afectados, sin errores. No se corrió
la batería de pruebas ni se reconstruyó/desplegó Docker. La migración 0004, flujo
de telemetría real, orden de snapshots y revisión visual quedan pendientes para
P4. Los fixtures de rollback/reintentos existentes se ajustaron para incluir las
tres métricas ahora necesarias en el historial; no se ejecutaron de nuevo.

### Inventario por fuentes y mantenimiento — 2026-10-27

Implementados estado actual, historial de diferencias, protección ante escaneo
incompleto/replay, API de búsqueda/paginación y vistas por equipo. Retención de
cambios (90 días, bajo bandera global existente) y tamaños físicos por relación.
Comprobaciones acotadas: cargo check Windows agente/servidor; TypeScript y ESLint
de archivos afectados. No se ejecutaron pruebas ni despliegues. Migración/flujo
real, UI y nuevo adaptador Linux pendientes de P4. Detalle en INVENTORY_STATE.md.

## Piloto local — 2026-10-28

- Proyecto Docker independiente `monitorv32-local` iniciado: server, dashboard, RabbitMQ, Redis, PostgreSQL/TimescaleDB y mantenimiento. Dashboard: http://127.0.0.1:18080. Se preservó el entorno anterior.
- Migraciones 1–11 aplicadas; Redis PONG; compilación Linux del servidor y frontend aprobada.
- `tests/local_pilot_smoke.py`: 10 comprobaciones aprobadas, incluyendo autenticación, RabbitMQ → DB → API, búsqueda/grupos, deduplicación por sesión, métricas diarias y políticas. Evidencia privada: `.local-test/logs/smoke.json`. Datos explícitamente sintéticos.
- Agente Windows release compilado; MSI 3.4.0 x64 construido con WiX 6.0.2. ZIP privado `.local-test/Monitor-Agent-Local-Windows.zip`, con instalación, reparación y eliminación coordinadas. Sin firma de producción.
- Pendiente: ejecutar el instalador en Windows, comprobar métricas reales y vista local, reinicio/recuperación, reparación/desinstalación y escenarios multisesión/Linux. No se declara P4 completo ni se instaló el agente en el anfitrión.

## Corrección del detalle y tema predeterminado — 2026-10-28

- Resuelto `programs.length` sobre undefined: el cliente agrupa filas reales de la API por hora, aplicación y ventana, preservando duraciones y recuentos. Se adaptaron también los nombres de campos de historial y actividad reciente.
- Tema oscuro por defecto cuando no existe preferencia guardada; Claro y Sistema siguen disponibles.
- Validación: build TypeScript/Vite aprobado; `node tests/desktop-contract.mjs` aprobado; `tests/local_desktop_smoke.py` aprobado con agente real (201 grupos históricos, 255 filas horarias, 12 aplicaciones en ese momento).
- Navegador: equipo afectado abre sin error, muestra Actividad (201), 48 controles para desplegar ventanas y permite expandirlas. Sin errores de consola en la verificación; tema oscuro confirmado. Dashboard local reconstruido y desplegado.
- Pendiente: contadores resumidos heredados de entrada/actividad no incluyen las nuevas sesiones; consultar el panel por sesión para esos valores. Continúan pendientes las pruebas generales de recuperación, reparación/desinstalación y Linux.

## 2026-10-29 — Configuración candidata (validación focalizada)

- `cargo test -p activity-monitor-server startup_config::tests --locked --offline`: 3 pruebas aprobadas; rechazo de modo desconocido, credenciales predeterminadas/bootstrap, secretos compartidos y ausencia de archivos obligatorios. Persisten advertencias previas no bloqueantes.
- Compose renderizado con proyecto independiente: ningún puerto publicado de postgres/rabbitmq/redis/server; dashboard limitado a 127.0.0.1; secretos directos retirados y cuatro archivos del servidor configurados. No se inició ni recreó ningún servicio.
- Generador Python parseado; no ejecutado en Linux ni generadas credenciales nuevas. Falta validación funcional del perfil, permisos efectivos y generación Linux.
