## 2026-11-07 — Respaldos cifrados y CI de navegador

Scripts age para DB/configuración, verificación de integridad y plantillas systemd implementados. Ensayos aislados aprobados: PostgreSQL 16 y Timescale/PostgreSQL 15, 5000 filas, chunks comprimidos, clave incorrecta, alteración, rutas inseguras, productor fallido y timeout. Guía ENCRYPTED_BACKUPS.md enlazada desde README/release. Copia externa, recuperación integral y RPO/RTO siguen pendientes.

PR #5 abierto: corregido runner del navegador a Ubuntu 24.04, Publication y retención aprobados; ciclo del agente y stack todavía en ejecución. No se fusiona hasta que todo el CI pase.

## 2026-11-07 — PR #4 fusionado; recuperación offline en curso

PR #4 fusionado en `e15e1103967b5c0fbd7cc497668d48551a0d4ec4`, cabeza `18154cbe9503bf683b80b2cde4f0bef134507014`, después de Publication preflight, Storage retention, Agent lifecycle y Stack smoke aprobados. Lapin 4.12.3 validado además con identidad/AMQPS aislados en RabbitMQ 3.12 y 4.2. Sin avisos de mantenimiento Rust; rsa opcional clasificado, sin supresión. Guía DevOps publicada; configuración/secretos reales se incorporan en destino.

Incremento local de recuperación: lotes de 256, presupuesto de 5 s, pausa corta mientras hay progreso y confirmaciones SQLite atómicas. Cinco regresiones de durabilidad aprobadas; ensayo final de 5000 eventos por AMQPS aprobado: mismos 5000 efectos y marcadores en PostgreSQL, cola vacía. Chromium también aprobó escape de metadatos no confiables y bloqueo de scripts inline con la CSP productiva. Nuevo incremento pendiente de publicación/CI. El servicio instalado continúa en 0.1.0; no se entrega paquete nuevo ni se modifica su ejecución.

## 2026-11-07 — PR #3 fusionado y preparación de release

PR #3 fusionado en `15a87c72b7cec92f74c17a995d30bf485aee2fa9` tras cuatro flujos de CI aprobados. Nuevo incremento local: Lapin 4.12.3 compila en Windows y pasó identidad/AMQPS contra RabbitMQ 4.2. Configuración TLS selecciona raíces nativas y ring sin activar defaults incompatibles. Auditoría sin avisos de mantenimiento; rsa sigue en lockfile opcional, sin ruta activa con `--target all`. CI Linux del nuevo incremento pendiente.

Redis retirado del código de despliegue y del contenedor del piloto; volumen conservado. El resto del piloto continúa saludable. Guía [RELEASE_DEVOPS.md](RELEASE_DEVOPS.md) preparada por pedido del usuario, con credenciales/destino a completar por DevOps. Renovación y rollback TLS preservan overlays, env-file y proyecto; dos regresiones focalizadas aprobadas.

## 2026-11-07 — Seguridad fusionada y candidato RabbitMQ validado

PR #2 fusionado en `ba7e3b37af68f005686f953ed0460978bf5aa833`; PR #1 ya estaba fusionado. Los cuatro flujos pasaron sobre `178f16c3`: Stack smoke (42336765435), Publication preflight (42336765482), Storage retention (42336765427) y Agent lifecycle (42336765440). Historial conservado, sin release ni reemplazo del piloto.

RabbitMQ 4.2 candidato pasó el recorrido aislado completo de identidad, MFA/CSRF/roles, enrolamiento, publicación nativa AMQPS, renovación, recuperación, revocación y rechazo de CA no confiable. La prueba permite elegir imagen mediante TEST_RABBITMQ_IMAGE y conserva el mismo código TOTP para verificar replay incluso al cruzar un intervalo temporal. Evidencia privada: `.local-test/security/rabbitmq-4.2-identity.log`. Migración real y durabilidad de colas pendientes; ver [plan](RABBITMQ_MIGRATION.md).

Continúa el bloque 1: revocación del token histórico, tratamiento de rsa y dependencias sin mantenimiento, bases del servidor/DB y decisión sobre Redis. Producción no aprobada; agente instalado 0.1.0 sin cambios.

## 2026-11-03 — Código publicado en Monitorv3/main

Commit publicado: 721723970d59b3f88451a13d96e0d3e6207fdd84. Árbol remoto verificado idéntico al preparado localmente; historial conservado y copia local sincronizada en main. Preflight remoto aprobado. No se creó release ni tag.

Primera ejecución de Storage retention: siete casos de almacenamiento aprobados; integración psql falló porque su fixture omitía 0010_input_quality. Se añadió esa migración al fixture y usuario explícito al healthcheck de PostgreSQL. La corrección no modifica el esquema del piloto, ya migrado. Resultado del nuevo CI pendiente de consultar; no se presenta toda la campaña como aprobada. Token histórico y aceptación productiva siguen pendientes.

## 2026-11-03 — Autorización de actualización del repositorio existente

Usuario autoriza subir el árbol actual limpio a Monitorv3/main conservando historial. No se crea repositorio nuevo ni se hace force-push. Token histórico y credenciales antiguas pendientes de revocación/rotación; no se afirma que estén invalidados. Preflight del árbol actual aprobado; credenciales, DB e instaladores privados excluidos. La subida no constituye release ni aprobación productiva. Ver GITHUB_PUBLICATION.md y el resultado de CI asociado al commit remoto.

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

> Plan vigente desde 2026-10-28: [TODO de preparación para producción](TODO.md). Sustituye los órdenes de trabajo históricos de este registro. Estado: piloto controlado; producción no aprobada.

# Estado vigente — 2026-10-28: implementación P2 cerrada; P4 iniciado

**Estimación global: alrededor de 90%. Implementación del alcance P0–P3: terminada,
con límites explícitos.** No significa 100% de funcionamiento ni aptitud para producción.
Se mantienen los pesos anteriores: P0 15%, P1 20%, P2 35%, P3 15%, P4 15%.
P4 conserva aproximadamente 35% de avance histórico; esta tanda incorpora sintaxis y
compilación, sin acreditar instalación, firmas, integración ni funcionamiento nativo.
Resultado orientativo 90,25%, redondeado a 90% (margen ±10 puntos).

P2 añadido: principales con claves/puertos independientes, atribución por ejecución y
panel/histórico separado; retención por lotes 30 días opt-in; diagnóstico de adaptadores;
recuperación de claves heredadas; herramienta de migración/purga; coordinadores de
sesiones con MSI/DEB y autostart por escritorio. Detalle: P2_IMPLEMENTATION_CLOSURE.md.

Límites explícitos: no coordenadas de heatmap (se retiró cuadrícula vacía ficticia),
Wayland sin acceso nativo informa no disponible, inicio automático mediante Explorer/XDG,
y no recuperación de cifrado sin clave original. No se trasladan como "pruebas pendientes"
para aparentar que esas funciones están implementadas.

Orden restante: compilación final/matriz → migraciones y concurrencia → servicios e
instaladores/rollback → conservación/restauración → UX, carga y piloto.
No se instalaron servicios ni aplicaron migraciones; no se publicó ni reemplazó GitHub.
Las secciones siguientes conservan la evolución histórica y no son el estado vigente.

---

# Estado vigente — 2026-10-28

**Estimación global: alrededor de 80%. Código P0–P3: alrededor de 90%.**
Margen orientativo ±10 puntos; estimación por alcance, no conteo de casillas.
Se conserva la ponderación original para evitar cambiar la medida entre entregas.

| Fase | Peso | Estado estimado |
| --- | ---: | --- |
| P0 — Despliegue base | 15% | 100% implementación; evidencia histórica |
| P1 — Datos y entrega | 20% | 100% implementación; integración reciente pendiente |
| P2 — Agente/ciclo de vida | 35% | 75% implementación |
| P3 — Dashboard/administración | 15% | 100% del alcance de interfaz documentado |
| P4 — Validación y entrega | 15% | 35% histórico; no acredita esta revisión |

Resultado ponderado orientativo 81,5%, redondeado a 80%; implementación 89,7%,
redondeada a 90%. No es una probabilidad de funcionamiento ni aptitud para producción.
P4 no avanzó en esta entrega: no hubo pruebas, builds, migraciones ni despliegues.

## Cierre P3

- Búsqueda/paginación del servidor, grupos, columnas, estados de conexión y versión objetivo.
- Administración: incorporación guiada, políticas completas globales/individuales con
  revisión y auditoría, confirmación por contenido reportado y seguimiento de versiones.
- Consulta admin de cuentas/roles/estado sin secretos. Edición de cuentas mediante archivo
  protegido existente; no se promete editor web ni aprovisionamiento remoto.
- Diagnóstico de captura separado de vida de tarea y fecha de última observación.
  Instrumentación USB/WiFi/inventario/aplicaciones; campos de recursos y entrada conservan
  sus diagnósticos específicos. Los adaptadores sin diagnóstico no se presentan sanos.
- Histórico de entrada con disponibilidad nullable y contadores de calidad conservados
  al compactar. Legado permanece desconocido. Migraciones 0009/0010 sin aplicar.
- Runbook y generador de manifiesto de release determinista; binarios reproducibles y
  firmas todavía no comprobados. Ver DASHBOARD_OPERATIONS.md.

## Continuación P2 — diagnóstico de sesión, 2026-10-28

Implementado diagnóstico periódico por IPC autenticado, consolidación en estado actual
sin historial nuevo y panel por instancia: entrada, pausa, pendientes, tareas y política.
Se corrigió el arranque tardío de captura cuando cambia la disponibilidad nativa sin
cambio de política. Ver SESSION_HEALTH.md. Sin pruebas, builds ni despliegues.
La estimación global se mantiene alrededor de 80%: este avance no cierra multiusuario,
instaladores ni migraciones. La concesión de un productor permanece intencionalmente.

## Qué sigue, en orden

1. Completar código P2 de multiusuario/RDP, salud consolidada de sesiones y errores de adaptadores.
2. Cerrar migración de instalaciones/claves antiguas, purga explícita e integración de
   sesiones/autostart y recuperación con MSI/DEB.
3. Ejecutar P4 sobre la revisión completa: compilación, contratos, migraciones, RabbitMQ,
   compactación/retención, permisos y UX; después servicios e instaladores nativos.
4. Carga, respaldo/restauración, piloto y release. La copia real de base sigue no disponible;
   las pruebas sintéticas no sustituyen esa comprobación.

No se ha publicado ni reemplazado el repositorio remoto. Los cambios siguen locales.
Los registros siguientes son históricos y no sustituyen este estado vigente.

---

# Estimación de avance — 2026-10-26

Estimación técnica por alcance, no conteo de casillas ni porcentaje de aptitud para
producción. Margen orientativo ±10 puntos. Se conserva la ponderación anterior.

| Fase | Peso total | Avance estimado de la fase |
| --- | ---: | ---: |
| P0: base del despliegue | 15% | 100% de implementación |
| P1: datos y entrega confiable | 20% | 50% |
| P2: agente y ciclo de vida | 35% | 40% |
| P3: dashboard y administración | 15% | 75% |
| P4: validación integrada y entrega | 15% | 35% |

Resultado ponderado: **aproximadamente 55% del alcance total** (55,5% aritmético,
no precisión real). Implementación P0–P3 alrededor del 60%, separando P4.

La estimación anterior era 45–50%. El avance reconoce enrolamiento inicial, correcciones
UX y evidencia nueva: compilación del agente Windows, lint/build del dashboard,
pruebas del servidor y almacenamiento, migraciones sintéticas comprimidas, imágenes
Docker actuales, integración RabbitMQ/API, Nginx con cambio de IP y recorrido visual
parcial. No implica que todo el agente ni los instaladores estén validados.

P0 conserva una validación pendiente con copia real no disponible. P1 ya implementa
atomicidad entre deduplicación y escritura, ACK tras commit y tres reintentos con
transferencia confirmada a cola de errores. Dos integraciones reales aprobaron
rollback, duplicados concurrentes, recuperación y conservación del mensaje fallido.
Quedan operación de la DLQ y ensayo de caída entre confirmación y ACK.
P2 concentra el mayor trabajo: recolectores por plataforma, sesiones, permisos,
revocación, empaquetado, actualización firmada, rollback y desinstalación.
P3 necesita capacidades/salud reales, administración y paginación del servidor.
P4 tiene evidencia actual en P4_VALIDATION.md, pero faltan pruebas multiplataforma,
fallos de disco/red/proceso, accesibilidad completa, carga, restauración y piloto.

## Orden vigente — corregido el 2026-10-27

Prioridad: completar implementación; las campañas integrales de P4 se retoman
cuando el bloque esté completo. La evidencia anterior se conserva, sin repetir
validaciones amplias después de cada modificación pequeña.

1. P2/P3: completar contrato de métricas y recolectores CPU, RAM, disco, red,
   uptime e inventario; persistir salud/capacidades y mostrar no disponible frente a cero.
2. P1: inventario por cambios, ventana de reenvío/deduplicación, conservación de
   seguridad, tamaños por tabla/chunk y operación/alertas de DLQ. Archivo externo
   solo si se identifica un uso concreto; no es requisito automático.
3. P2: revocación y reparación de sesiones, rotación de claves sin perder pendientes,
   multiusuario/RDP, migración de instalaciones antiguas y cierre nativo restante.
4. P2: MSI/DEB, autostart de sesión, reparación/reinstalación, actualización firmada,
   recuperación de actualización interrumpida y desinstalación completa de sesiones.
5. P3: administración de incorporación, políticas y versiones; grupos, columnas,
   búsqueda/paginación del servidor y correcciones UX pendientes.
6. P4: validación integral del bloque implementado, servicios reales SCM/systemd,
   instalación/actualización/desinstalación, carga, restauración, accesibilidad y piloto.

Comprobaciones focalizadas únicamente cuando sean necesarias para el cambio.
El porcentaje de arriba es la última estimación histórica, no un avance recalculado
por cantidad de pruebas. Las tareas abiertas del TODO siguen siendo trabajo de código.

## Evidencia nueva del bloque 1–2

El núcleo del punto 1 está implementado y validado con PostgreSQL y RabbitMQ.
El punto 2 sigue en curso: supervisor, tareas auxiliares e IPC cancelables; políticas
persistentes y dinámicas; caché que conserva pendientes ante errores y saturación.
Windows: 9 pruebas focalizadas aprobadas. Debian 12: compilación del binario de
pruebas y 10 pruebas aprobadas, incluida SIGTERM. Esto no valida todavía servicios
instalados, escritorio real ni todos los recolectores. Ubuntu 24.04 también compiló
y aprobó las 10 pruebas con sus propias bibliotecas y caché independiente.
La estimación global de 55% sigue siendo la última referencia de alcance; no se
recalcula automáticamente por cada prueba aprobada.

Continuación: servidor actualizado en el stack y smoke aprobado. Web local
cancelable y hooks/tray Windows con señal de cierre. El proceso real del agente
aprobó parada SIGTERM, reinicio tras SIGKILL e integridad offline en Debian/Ubuntu
bajo Docker. Quedan servicios instalados SCM/systemd y escritorio real; no se
marca el punto 2 completo solamente por estas pruebas.


### Ciclo administrativo — 2026-10-27

Implementados scripts locales Linux/Windows de actualización con hash y rollback
ante fallo de arranque, exclusión mutua y desinstalación conservando datos. Retirados
autoactualizador del repositorio antiguo y autodesinstalación por batch destructivo.
Pruebas aprobadas con filesystem real y gestores de servicio simulados: Windows,
Debian y Ubuntu. Detalles y comandos en PLATFORM_LIFECYCLE.md. Pendientes servicios
reales, MSI/DEB, firmas, reparación/reinstalación, sesiones y recuperación ante corte
de energía durante una actualización. Sin cambios publicados en GitHub.

### Código de métricas/capacidades/salud — 2026-10-27

Implementado el contrato de estado actual completo entre agente, RabbitMQ, SQL,
API y dashboard; disco/red/uptime, RAM corregida, inventario básico y diagnóstico
por contexto/política/supervisión. Dos filas por equipo, sin historial adicional.
Integración y despliegue diferidos a P4. Detalle en TELEMETRY_CONTRACT.md.
Siguiente bloque de código: inventario por estado/cambios y mantenimiento P1;
no repetir la campaña de servicios antes de completar esas implementaciones.

### Inventario por fuentes y mantenimiento — 2026-10-27

Implementados estado actual, historial de diferencias, protección ante escaneo
incompleto/replay, API de búsqueda/paginación y vistas por equipo. Retención de
cambios (90 días, bajo bandera global existente) y tamaños físicos por relación.
Comprobaciones acotadas: cargo check Windows agente/servidor; TypeScript y ESLint
de archivos afectados. No se ejecutaron pruebas ni despliegues. Migración/flujo
real, UI y nuevo adaptador Linux pendientes de P4. Detalle en INVENTORY_STATE.md.

## 2026-10-27 — Conservación por investigación

Implementados migración 0006, API de investigaciones y limpieza de seguridad opt-in con protección por equipo/período. Apertura y limpieza serializadas; liberación conserva el motivo. Sin ejecutar pruebas, builds ni migraciones. Faltan panel, permisos por rol y continuación de P1/P2/P3; ver SECURITY_RETENTION.md.

## 2026-10-27 — Continuación de implementación P1/P3

- Investigaciones: interfaz completa de apertura/listado/liberación, filtros y paginación; atribución al JWT mediante migración 0008. Bloqueo de apertura acotado a cinco segundos.
- DLQ: API/panel de estado y reenvío por lotes, auditoría de intención/resultado, bloqueo concurrente, confirmación de publicación antes del ACK y conservación de mensajes inválidos/vencidos.
- Ventana de reenvío fija de 30 días; marcas conservadas al menos 32 días desde recepción; índice 0007 y tarea de limpieza opt-in. No habilitada en despliegue.
- Almacenamiento: consulta paginada de relaciones y totales Timescale por hypertable.
- UX: retirada desinstalación remota no soportada, bloqueo de comandos remotos update/uninstall, título de login, claves React de filas de seguridad y errores HTTP de consulta en lugar de listas vacías.

No se ejecutaron pruebas, builds, migraciones ni despliegues. Pendientes principales de código: modelo multiusuario/permisos; P2 (reparación/revocación/rotación, sesiones y empaquetado); P3 (grupos/columnas/búsqueda de flota en servidor, incorporación/políticas/versiones). Diagnóstico del aviso de workers Timescale requiere entorno de P4; no se da por resuelto mediante cambios especulativos.

## 2026-10-27 — Ciclo de vida de sesiones y reparación P2

Implementados comandos administrativos Windows/Linux para activate/repair/rotate/revoke/reenroll del mismo principal. Enrolamiento nuevo separa secreto IPC y secreto de outbox; rotación de legado fija el secreto anterior de caché antes de cambiar IPC. Journal privado recuperable, exclusión de operaciones y bloqueo de arranque del agente ante journal pendiente. Los scripts preservan identidad, outbox y configuración; rechazan claves ambiguas y migraciones no inferibles.

Autostart opt-in y retirada: unidad systemd de usuario con condición por UID, tarea Windows Interactive/Limited con recuperación y salida del launcher propagada. Reparación de servicio Linux desde artefacto/hash y registro Windows con -Repair, sin cambiar rutas ni iniciar procesos.

No se ejecutaron scripts, pruebas, builds, migraciones ni instalaciones. Pendientes: MSI/DEB y firmas, multiusuario/RDP, captura nativa Linux/cierre modal, migraciones antiguas/purga explícita, recuperación de actualización binaria y pendientes P3. Instrucciones y límites en SESSION_LIFECYCLE.md y PLATFORM_LIFECYCLE.md.

## 2026-10-27 — Cierre de código P1 y continuación P2; vista personal

P1: implementados roles/cuentas Argon2id y diagnóstico de capacidad de workers. Queda validación integrada P4, incluido el incidente real de columnstore; no se afirma resuelto.

P2: listener X11 en proceso cancelable, retirada de modal Windows, definición/generadores DEB/MSI, verificación de firmas locales y journal/recuperación de actualización. P2 NO está completo: siguen abiertos multiusuario/RDP y salud consolidada, migración de legado/claves ambiguas, purga explícita e integración MSI/sesiones.

Solicitud adicional incorporada: Mi equipo, interfaz local de solo lectura con recursos/estado y protección de origen/loopback. Sin comandos, credenciales ni datos de otros equipos. Textos del dashboard cambiados de flota a Equipos de oficina. Ver LOCAL_VIEW.md.

No se ejecutaron pruebas, builds, paquetes, instaladores, migraciones ni despliegues. Los archivos fuente y scripts están preparados; no equivalen a una release probada o firmada.

## Piloto local — 2026-10-28

- Proyecto Docker independiente `monitorv32-local` iniciado: server, dashboard, RabbitMQ, Redis, PostgreSQL/TimescaleDB y mantenimiento. Dashboard: http://127.0.0.1:18080. Se preservó el entorno anterior.
- Migraciones 1–11 aplicadas; Redis PONG; compilación Linux del servidor y frontend aprobada.
- `tests/local_pilot_smoke.py`: 10 comprobaciones aprobadas, incluyendo autenticación, RabbitMQ → DB → API, búsqueda/grupos, deduplicación por sesión, métricas diarias y políticas. Evidencia privada: `.local-test/logs/smoke.json`. Datos explícitamente sintéticos.
- Agente Windows release compilado; MSI 3.4.0 x64 construido con WiX 6.0.2. ZIP privado `.local-test/Monitor-Agent-Local-Windows.zip`, con instalación, reparación y eliminación coordinadas. Sin firma de producción.
- Pendiente: ejecutar el instalador en Windows, comprobar métricas reales y vista local, reinicio/recuperación, reparación/desinstalación y escenarios multisesión/Linux. No se declara P4 completo ni se instaló el agente en el anfitrión.

## Corrección del piloto: escritorio y apariencia — 2026-10-28

- El agente nativo instalado reporta actividad y snapshots de aplicaciones en `session_events`; las pantallas anteriores consultaban exclusivamente tablas heredadas.
- Migración 0012: vistas de lectura para actividad y último snapshot por usuario, sin duplicar almacenamiento ni modificar la retención. Historial, actividad reciente, aplicaciones abiertas y rankings consultan ambas generaciones. Tiempo de aplicaciones representa tiempo por persona, no encendido del equipo.
- Selector de tema Sistema/Claro/Oscuro en acceso y cabecera; persistencia local, seguimiento del sistema y sincronización entre pestañas. Paleta compartida en páginas, tablas y gráficos.
- Pendientes generales: reinicios/recuperación, reparación/desinstalación, sesiones simultáneas y Linux. La captura de mapas de calor sigue fuera del alcance implementado.

## Corrección del detalle y tema predeterminado — 2026-10-28

- Resuelto `programs.length` sobre undefined: el cliente agrupa filas reales de la API por hora, aplicación y ventana, preservando duraciones y recuentos. Se adaptaron también los nombres de campos de historial y actividad reciente.
- Tema oscuro por defecto cuando no existe preferencia guardada; Claro y Sistema siguen disponibles.
- Validación: build TypeScript/Vite aprobado; `node tests/desktop-contract.mjs` aprobado; `tests/local_desktop_smoke.py` aprobado con agente real (201 grupos históricos, 255 filas horarias, 12 aplicaciones en ese momento).
- Navegador: equipo afectado abre sin error, muestra Actividad (201), 48 controles para desplegar ventanas y permite expandirlas. Sin errores de consola en la verificación; tema oscuro confirmado. Dashboard local reconstruido y desplegado.
- Pendiente: contadores resumidos heredados de entrada/actividad no incluyen las nuevas sesiones; consultar el panel por sesión para esos valores. Continúan pendientes las pruebas generales de recuperación, reparación/desinstalación y Linux.

## UX del detalle del equipo — 2026-10-28

- Entrada predeterminada en Resumen: identidad compacta, cuatro indicadores de recursos/salud, última actividad reportada, cinco aplicaciones con más uso y accesos directos. Retirados los contadores heredados engañosos del encabezado.
- Secciones independientes: Resumen, Actividad, Aplicaciones, Recursos, USB, WiFi y Diagnóstico. El diagnóstico y las sesiones ya no aparecen delante de la actividad.
- Actividad agrupada por aplicación, búsqueda por aplicación/título, filtro por hora, ocho aplicaciones por página y ventanas desplegables (diez iniciales, cargar más). Se conservan tiempos por persona y se distinguen del tiempo encendido.
- Recursos: métricas actuales separadas de historia; almacenamiento/red desplegables. Fechas de actividad inicializadas con el calendario local.
- Verificación: build TypeScript/Vite y contrato de agrupación aprobados. Navegador con equipo real: resumen carga, actividad limita a ocho filas y búsqueda Excel reduce a una aplicación.
- Pendiente general: consolidación de contadores de entrada por sesión, ciclo de reparación/desinstalación y recuperación, escenarios Linux/multisesión.

## 2026-10-29 — Bloque 1: perfil candidato y configuración estricta

Añadido overlay separado sin puertos internos publicados y con secretos por archivo; Redis optativo. Generador Linux de configuración privada, sin iniciar servicios ni reemplazar archivos. Validación de configuración antes de conectar DB/escuchar API; producción exige cuentas administradas y CORS del mismo origen. Límites iniciales de servidor/dashboard/mantenimiento.

No se modificó el piloto en ejecución ni el agente. Pendientes: despliegue funcional Linux, TLS/identidad por agente, roles DB, UID/permisos y resto de hardening. README y guía de despliegue actualizados; bloque 1 parcial.


## 2026-11-03 — Limpieza del árbol heredado

Retirados 73 archivos obsoletos y los crates manager/remover. Ajustados Cargo.lock y tres Dockerfiles; documentación vigente enlazada desde README. Ver [REPOSITORY_LAYOUT.md](REPOSITORY_LAYOUT.md) para inventario, reemplazos y excepciones. Metadata del workspace, preflight y diff aprobados; resolución completa offline limitada por adler2 ausente. Sin reinstalación, reinicio ni cambios a datos del piloto. Esta limpieza no cierra los bloques pendientes de producción.


## 2026-11-03 — Bloque 8: implementación UX

Cargas independientes y estados por consulta, filtros persistentes, tablas paginadas, diagnóstico desplegable, ayuda integrada, accesibilidad estructural y permisos comprensibles. Ver [DASHBOARD_UX.md](DASHBOARD_UX.md). Build TypeScript/Vite y ESLint focalizado; regresión Chromium sintética con revisión visual de capturas, ambos temas y tres tamaños. El bloque 9 conserva aceptación humana, lector de pantalla real e integración multiplataforma. No se modificó la versión del agente ni se desplegó el dashboard sobre Docker.


## 2026-11-04 — Ventanas del piloto atrasadas por servidor detenido

Docker Desktop estaba sin motor Linux disponible; servicio y sesión del agente seguían activos. Al iniciar Docker se confirmó captura reciente y reenvío FIFO de la cola persistente. Pendientes del servicio: 5.439 → 0; API final: 12 aplicaciones recientes a las 20:03:37 UTC, última instantánea 20:03:05 UTC. No se modificó ni reinstaló el agente. Registrada mejora de tiempo de drenaje en bloque 6 y guía [PILOT_TROUBLESHOOTING.md](PILOT_TROUBLESHOOTING.md). No equivale a certificar todos los escenarios de desconexión prolongada.
## 2026-11-06 — Seguridad: endurecimiento y revisión focalizada

Matriz aislada de sesiones/MFA/roles/CSRF aprobada, 7 pruebas unitarias de configuración/rutas aprobadas y prueba Nginx ampliada aprobada (HTTPS, archivos sensibles, límite de login, registros sin query). Build frontend/servidor aprobado; lint global conserva un fallo en AdministrationPage.tsx. npm audit sin avisos conocidos después de correcciones. Escaneo Rust e imágenes detectó pendientes; token histórico sin revocación confirmada. Informe completo: [SECURITY_REVIEW_2026-11-06.md](SECURITY_REVIEW_2026-11-06.md). Cambios locales sin despliegue/publicación. Producción no aprobada.
## 2026-11-06 — PR de seguridad y corrección del fixture Windows

Avance publicado en PR #2, commit 59a3842, árbol idéntico al comprobado localmente. Recorrido completo de identidad/transporte aprobado con reqwest nuevo; lint y build dashboard aprobados. Publicación preflight y almacenamiento aprobados en CI. Ciclo de vida Windows detectó un fixture que perdía PSScriptRoot: se reemplazan scriptblocks por copias reales, estado simulado compartido y carpeta con ACL protegida. La prueba requiere administrador porque los permisos son reales; la sesión local no puede asignar propietario Administrators. Validación de esa corrección reservada al runner Windows, sin reducir controles productivos. PR sin fusionar hasta CI satisfactorio; producción y piloto sin actualizar.

