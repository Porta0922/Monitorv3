# Monitor — Supervisión de equipos de oficina

**Estado: candidato 0.2.3 sin firma; todavía no aprobado para producción.**

[Actualizaciones silenciosas Windows, releases e icono de conexión](docs/AUTOMATIC_UPDATES.md).

[Entrega e instalación 0.2.1](docs/RELEASE_0_2_1.md).

Aceptación del 2026-11-20: reparado el coordinador MSI y comprobada la reparación
del agente instalado conservando configuración y reanudando servicio/sesión.
También pasó el drenaje integrado de 5.000 eventos por AMQPS sin duplicar efectos
en DB. Entrega 0.2.1: incluye las correcciones del instalador. La firma se pospone por
decisión del responsable; siguen pendientes las validaciones finales de producción. Ver [estado del agente](docs/AGENT_RELEASE_0_2.md).

[Guía paso a paso de release para DevOps](docs/RELEASE_DEVOPS.md): dónde configurar cuentas, MFA, secretos, certificados, red, enrolamiento, actualización y reversión. Las credenciales reales se incorporan en el servidor de destino.

Estado actualizado al **2026-11-17**: agente piloto **0.2.0** instalado. Existen
candidatos MSI Windows y DEB amd64 para Debian 12, Ubuntu 22.04 y 24.04;
la firma y la aceptación final por plataforma siguen pendientes.

El dashboard corregido pasó el escaneo sin hallazgos; la imagen candidata de
PostgreSQL/Timescale no presenta críticos ni altos y conserva dos hallazgos de
severidad no determinada. No se sustituyeron las imágenes del piloto.
La recuperación sintética, la migración PG15→PG17 y el ensayo de transferencia/
reversión RabbitMQ pasaron. La aceptación integrada con agentes instalados,
los certificados/credenciales del destino y el correo siguen pendientes.

Consultar [TODO vigente](docs/TODO.md), [seguridad de imágenes](docs/IMAGE_SECURITY.md),
[alertas](docs/EXTERNAL_ALERTS.md) y [respaldos](docs/ENCRYPTED_BACKUPS.md).
Linux admite metadatos de ventanas X11; Wayland no admite captura global.
RabbitMQ 4.2 pasó el recorrido aislado de identidad/TLS, pero aún no reemplaza
el broker del piloto: [migración](docs/RABBITMQ_MIGRATION.md).

Monitor reúne recursos, actividad de aplicaciones, inventario y eventos de computadoras de oficina. El agente se instala en cada equipo; el servidor central recibe la información y el dashboard permite consultarla. RabbitMQ se mantiene como transporte de eventos. La recuperación offline del servicio usa lotes acotados y acelera el reenvío mientras haya pendientes; conserva IDs para deduplicación. [Garantías y límites](docs/TELEMETRY_DELIVERY.md).

La nueva numeración comienza en 0.1.0. El piloto local fue reinstalado desde cero por autorización del usuario, descartando los datos 3.4.0. El MSI nuevo es privado y sin firma; los artefactos antiguos 3.x no deben reutilizarse. Esto no valida la migración que conserva datos de otras instalaciones. Consulta [versionado](docs/VERSIONING.md).

## Arquitectura

El inicio del dashboard se centra en los equipos: búsqueda, estado de conexión,
última señal y acceso al detalle. Wi-Fi, USB y aplicaciones se consultan dentro
de cada equipo; ya no forman un grupo independiente en el menú lateral.
Las alertas y métricas generales conservan sus propias páginas.

```text
Computadora supervisada
  Servicio nativo: recursos, inventario, conectividad y cola persistente
  Sesión de usuario: aplicación enfocada, ventanas y contadores de entrada
          │ IPC local cifrado y autenticado
          ▼
  Servicio → RabbitMQ → Servidor Rust → PostgreSQL / TimescaleDB
                            ▲                    ▲
                            │ API                │ compactación / retención
                  Nginx + Dashboard         db-maintenance

  Vista local «Mi equipo»: http://127.0.0.1:9876 (solo lectura)
```

| Componente | Responsabilidad | Ubicación |
|---|---|---|
| Agente Rust | Captura según contexto/permisos, identidad y caché offline cifrada | Equipo Windows o Linux |
| RabbitMQ | Entrega de eventos y colas de errores/reintento | Servidor, Docker |
| Servidor Rust/Axum | Ingestión, políticas, autenticación y API | Servidor, Docker |
| PostgreSQL/TimescaleDB | Estado actual, eventos y resúmenes históricos | Servidor, volumen persistente |
| Dashboard React/TypeScript | Consulta y administración mediante Nginx | Servidor, Docker |
| Mantenimiento | Compactación y eliminación por lotes configurables | Servidor, Docker |

El servicio no captura el escritorio desde la sesión de sistema. Esa función corresponde al componente de usuario enrolado. Los tiempos de varias personas/sesiones no representan horas de encendido de una máquina.

## Qué permite consultar

- Resumen del equipo: conexión, CPU, memoria, tiempo encendido, estado del agente y última actividad reportada.
- Actividad por aplicación y título de ventana, búsqueda, filtro por hora, paginación y detalles desplegables.
- Último reporte de aplicaciones abiertas, inventario y cambios, USB y conexiones WiFi.
- Recursos actuales e históricos, disponibilidad de recolectores y diagnóstico por sesión.
- Grupos, políticas globales/por equipo, versiones reportadas, alertas e investigaciones.
- Tema oscuro predeterminado, con opciones Claro y Sistema y preferencia local persistente.
- Cargas independientes y reintentos por sección, tablas paginadas y filtros conservados en enlaces; ayuda integrada desde la barra superior. Ver [UX y límites de validación](docs/DASHBOARD_UX.md).
- Vista local del equipo, sin administración de otras computadoras.

La última actividad es un reporte, no una transmisión del escritorio. Las muestras ausentes o antiguas deben distinguirse de un valor cero. La cobertura depende del sistema operativo, los permisos y la política de recolección.

## Plataformas y límites actuales

| Destino | Estado |
|---|---|
| Windows x64 | Agente nativo instalado y flujo real comprobado en el piloto local; ciclo completo de recuperación/actualización todavía pendiente |
| Ubuntu / Debian | Código y herramientas de empaquetado/ciclo de vida preparados; falta completar la matriz nativa de certificación |
| Linux X11 | Implementación de captura; permisos, recuperación y escritorios deben verificarse en los destinos soportados |
| Wayland | No se promete captura equivalente a X11; capacidades no disponibles deben indicarse explícitamente |
| macOS | No forma parte de la entrega actual, aunque existan referencias/código heredados |

Pendientes relevantes: validar el despliegue de identidades por equipo en las plataformas objetivo, preparar despliegue productivo, firmas, backup/restore y validación multiplataforma. Los mapas de calor por coordenadas no están habilitados. La autoactualización remota heredada está deshabilitada.

El alcance completo y los criterios de aceptación están en [TODO de producción](docs/TODO.md).

## Estructura del repositorio

Los instaladores y utilidades del proyecto anterior fueron retirados. Consulta el [mapa vigente y las excepciones de compatibilidad](docs/REPOSITORY_LAYOUT.md).

```text
agent/                    Agente nativo, tareas, IPC, caché y vista local
server/                   API, consumidores y migraciones SQLx
server/migrations/        Evolución versionada de la base
dashboard/               Aplicación web y Nginx
packaging/windows/        MSI y herramientas administrativas Windows
packaging/linux/          DEB, systemd y herramientas Linux
packaging/common/         Migración y mantenimiento de datos locales
scripts/                  Mantenimiento de DB, releases y piloto local
tests/                    Contratos, integración y escenarios de ciclo de vida
docs/                     Plan, operación, arquitectura y evidencia
.local-test/              Archivos privados del piloto, ignorados por Git
```

## Ejecutar el piloto local

Si las ventanas aparecen antiguas, consulta el [diagnóstico del piloto y recuperación de eventos](docs/PILOT_TROUBLESHOOTING.md).

Requisitos: Docker Engine/Compose v2 con soporte para `!override` y construcción de imágenes Linux. En Windows, Docker Desktop debe estar en ejecución. Las versiones de Rust y dependencias se fijan en `rust-toolchain.toml`, Cargo.lock, Dockerfiles y el lockfile del dashboard.

El perfil local usa un proyecto Docker independiente y puertos de loopback. Para una instalación nueva, crear `.local-test/.env` con estos campos y valores aleatorios propios:

```dotenv
ADMIN_USERNAME=admin
ADMIN_PASSWORD=<contraseña única>
JWT_SECRET=<secreto aleatorio>
AGENT_AUTH_TOKEN=<secreto aleatorio del piloto>
LOCAL_DB_PASSWORD=<secreto hexadecimal aleatorio>
LOCAL_RABBIT_PASSWORD=<secreto hexadecimal aleatorio>
AGENT_CONFIG_DISABLED_MONITORS=heatmap
```

Generar cada secreto por separado, por ejemplo con `python -c "import secrets; print(secrets.token_hex(32))"`. Reemplazar todos los marcadores, restringir permisos del archivo y no incorporarlo a Git. Para el piloto existente, conservar sus valores: cambiarlos sin rotación coordinada puede impedir el acceso a volúmenes ya inicializados.

Desde la raíz del repositorio:

```powershell
docker compose --project-name monitorv32-local --env-file .local-test/.env -f docker-compose.yml -f docker-compose.local.yml up -d --build
```

| Acceso local | Dirección |
|---|---|
| Dashboard y API vía Nginx | http://127.0.0.1:18080 |
| API directa del piloto | http://127.0.0.1:18000 |
| RabbitMQ administración | http://127.0.0.1:15675 |
| RabbitMQ AMQP | 127.0.0.1:5673 |
| PostgreSQL | Interno a Docker: `postgres:5432`, sin puerto publicado |

Credenciales nuevas del dashboard en `.local-test/ACCESO-LOCAL.md`, archivo privado que no se distribuye. La cuenta bootstrap de este piloto local no tiene MFA: dejar vacío el código de autenticación. Las cuentas productivas con MFA siguen exigiendo el código. El paquete usado está en `.local-test/candidate-0.1.0/`; no redistribuirlo porque contiene configuración privada.

Para consultar, detener o reanudar el piloto ya preparado:

```powershell
./scripts/local-pilot.ps1 -Action status
./scripts/local-pilot.ps1 -Action stop
./scripts/local-pilot.ps1 -Action start
```

`start` no reconstruye imágenes; después de cambiar código, usar el comando Compose con `--build`. No usar `down -v` como procedimiento normal: elimina los volúmenes. Las migraciones SQLx se aplican al arrancar el servidor; respaldar antes de actualizar una instalación con datos.

El Compose base contiene configuración de desarrollo y no debe exponerse directamente a una red de producción. El perfil local no configura HTTPS/AMQPS ni sustituye al futuro perfil productivo.

## Perfil candidato de producción

Existe un [overlay de producción en preparación](docs/PRODUCTION_DEPLOYMENT.md), separado del piloto: roles PostgreSQL separados, trabajo de migraciones independiente, servidor/dashboard sin root, HTTPS en loopback 18443 y AMQPS en loopback 25671. Usa secretos desde archivos y validación estricta al arrancar. Redis fue retirado: no lo utiliza la aplicación. El generador requiere Linux, cuentas Argon2id y certificados válidos; ver la guía para nombres, CA y permisos.

**No está listo para exposición real:** faltan certificados reales y su renovación operativa, aceptación multiplataforma y validación funcional completa del perfil, entre otros criterios del TODO. No aplicar el overlay sobre los volúmenes del piloto ni regenerar sus credenciales.

## Instalar el agente

El agente se ejecuta fuera de Docker, dentro de la computadora que se supervisa. Necesita la URL del servidor, acceso al broker y configuración de identidad/caché/sesión. Plantilla: [packaging/agent.env.example](packaging/agent.env.example). Para el candidato productivo, seguir [enrolamiento y custodia](docs/AGENT_IDENTITY.md) y usar `scripts/configure-agent-identity.py` sobre una instalación preparada; conserva UUID y caché y no reinicia el servicio.

En el piloto Windows ya preparado:

1. Extraer completamente el ZIP privado del instalador.
2. Ejecutar `InstalarLocal.cmd` y aceptar la elevación administrativa.
3. Esperar los primeros reportes y consultar el dashboard o `http://127.0.0.1:9876`.
4. Usar los auxiliares `RepararLocal.cmd` y `DesinstalarLocal.cmd` del mismo paquete. La eliminación normal conserva los datos documentados.

El ZIP del piloto contiene credenciales y un MSI sin firma de producción: no es un instalador genérico redistribuible. No ejecutar directamente el MSI omitiendo su coordinador. Las direcciones de loopback solo sirven cuando servidor y agente están en la misma computadora.

Para generar nuevos paquetes y configurar otros equipos, seguir [distribución de paquetes](docs/PACKAGE_DISTRIBUTION.md), [cierre de implementación y límites](docs/P2_IMPLEMENTATION_CLOSURE.md) y [versionado](docs/VERSIONING.md). La distribución firmada y la transición 3.x → 0.1.x siguen pendientes de validación; no forzar el downgrade de MSI.

## Desarrollo y comprobaciones focalizadas

```powershell
cargo check --workspace --locked
cargo build --release -p activity-monitor-agent --locked
npm ci --prefix dashboard
npm run build --prefix dashboard
node tests/desktop-contract.mjs
```

El agente informa la versión de `agent/Cargo.toml`. No cambiar solo el nombre del instalador ni empaquetar un binario antiguo como una versión nueva. Para MSI se requieren WiX y su extensión Util; para DEB, entorno Linux y herramientas de empaquetado. Consultar los scripts de `packaging/` y sus requisitos antes de construir.

Comprobaciones del piloto preparado:

```powershell
python tests/local_pilot_smoke.py
python tests/local_desktop_smoke.py
```

La primera prueba publica eventos sintéticos identificados y modifica los metadatos/política de su equipo de prueba. La segunda consulta datos reales de un agente ya instalado. Ambas usan exclusivamente el proyecto `monitorv32-local` y su configuración privada; no son una certificación productiva ni se ejecutan contra otro entorno por cambiar una URL.

## Seguridad y datos

Implementado en código: sesiones opacas revocables mediante cookies HttpOnly/Secure/SameSite en producción, protección CSRF, MFA TOTP obligatorio para administradores, límites de login/API, auditoría y cuentas individuales con Argon2id y roles comprobados en servidor, separación de acceso agente/administrador, caché AES-256-GCM, IPC local autenticado/cifrado y vista local de solo lectura con controles de origen.

Identidad individual HTTP y RabbitMQ: enrolamiento de un solo uso, permisos de publicación por UUID, renovación automática, revocación con cierre de conexiones y credenciales locales cifradas. El dashboard permite administrar el acceso en Equipo → Diagnóstico. Producción rechaza tokens compartidos y transporte inseguro; desarrollo conserva compatibilidad. Ver [identidad del agente](docs/AGENT_IDENTITY.md).

Limitaciones pendientes: certificados reales, operación de su renovación y aceptación del despliegue TLS, firma de releases, completar controles de privilegios del despliegue y validación integrada. El cifrado de la caché no protege automáticamente el tráfico de red ni la base central. Ver [plan de seguridad y producción](docs/TODO.md).

Retención configurable: detalle de métricas 14 días y resúmenes 365 como valores de partida; no representan una política aprobada para cualquier organización. La eliminación histórica general permanece desactivada por defecto. Retención/compactación no equivale a backup. Revisar [retención](docs/DATA_RETENTION.md), [migraciones](docs/MIGRATIONS.md) y los criterios de restauración del TODO antes de habilitar borrado.

## Documentación vigente y versiones

Operación preparada: detector periódico de salud/colas/actividad sospechosa, recuperación aislada de DB/configuración y despliegue candidato PG17/RabbitMQ 4.2 con volúmenes nuevos. Código local, todavía sin activar ni ejecutar la campaña integral. Guías: [alertas](docs/EXTERNAL_ALERTS.md) y [recuperación e imágenes](docs/PRODUCTION_RECOVERY.md).

Preparación de entrega: [firma y confianza](docs/RELEASE_SIGNING.md) y [alertas externas por fallos de backups](docs/EXTERNAL_ALERTS.md). Implementación local, pendiente de credenciales, activación y aceptación por DevOps. No se enviaron alertas ni se firmaron/publicaron paquetes.

Agente **0.2.0** instalado en el piloto Windows por autorización del usuario; configuración conservada y servicio/sesión relanzados. MSI genérico y asistente Windows preparados; DEB con dependencias calculadas y construcción aislada por distribución. Las pruebas integrales siguen diferidas hasta completar paquetes. Sin nueva publicación PR ni aprobación productiva. Ver [preparación y pendientes del instalador](docs/AGENT_RELEASE_0_2.md).

- [TODO de producción](docs/TODO.md): prioridades, tareas y criterios de cierre.
- [Progreso y evidencia](docs/REFACTOR_PROGRESS.md): registro histórico, con fecha y alcance.
- [Validación P4](docs/P4_VALIDATION.md): resultados y pendientes de comprobación.
- [Versionado](docs/VERSIONING.md): serie 0.x, transición y preparación de releases.
- [CHANGELOG](CHANGELOG.md): cambios por versión, separados del historial heredado.
- [UX del dashboard](docs/DASHBOARD_UX.md), [roles](docs/ACCESS_CONTROL.md) y [vista local](docs/LOCAL_VIEW.md).

**Regla de mantenimiento:** actualizar este README en el mismo conjunto de cambios que modifique comportamiento, requisitos, instalación, configuración, versiones o limitaciones. Mantener TODO y CHANGELOG alineados. Una función preparada no se documenta como validada, ni una prueba local como certificación de producción.

Los documentos antiguos pueden contener estados anteriores; prevalecen el plan vigente y la evidencia más reciente identificada. La primera versión estable del agente será 1.0.0 solo tras aprobar la salida a producción.

### Acceso humano — bloque 3

El candidato incluye la pantalla **Sesiones y acceso**, revocación propia/administrativa, recuperación MFA mediante `scripts/manage-accounts.py`, CSP y CSV protegido ante fórmulas. Producción requiere `MONITOR_PUBLIC_ORIGIN` con el origen HTTPS exacto. Las cuentas se administran en el anfitrión Linux; contraseñas y semillas no se gestionan desde el navegador. Procedimientos y límites: [ACCESS_CONTROL.md](docs/ACCESS_CONTROL.md). Migración nueva: `0015_human_sessions.sql`. Desplegar servidor, migración y dashboard coordinadamente; los JWT antiguos del navegador dejan de ser válidos en producción. El piloto limpio ya incluye esta migración; utiliza bootstrap local sin MFA, no el perfil de cuentas productivas.

### Datos por sesión — bloque 4

El resumen del equipo muestra contadores diarios UTC por persona y sesión, con desglose plegable y CSV para operador/administrador. Sus tiempos son horas-persona, no tiempo encendido. Los totales abarcan todas las páginas y solo incluyen contadores con captura disponible; sin muestras se muestra ausencia de datos.

Las ventanas se presentan como **últimas ventanas reportadas**, con fecha, principal, sesión y vigencia. Una observación reciente tampoco confirma que sigan abiertas. Requiere migración `0016_running_apps_scope.sql` y actualización coordinada servidor/dashboard. El piloto limpio ya incluye las migraciones 0016 y 0017. El cierre del candidato incorpora también el resumen global, compactación por sesión (migración 0017) y snapshot vacío al cierre ordenado; ante caída abrupta se informa antigüedad. Ver alcance y evidencia en [BLOCK4_CLOSURE.md](docs/BLOCK4_CLOSURE.md).

### Bloque 4 y piloto actual

El candidato unifica contadores por persona en resumen global/equipo, histórico y CSV mediante la migración 0017. Los días de informes son UTC y los datos sin captura no son ceros. Compactación de sesiones, snapshots al cierre, filtros, políticas y comandos de diagnóstico tienen comprobaciones focalizadas. Alcance, exclusiones y evidencia: [BLOCK4_CLOSURE.md](docs/BLOCK4_CLOSURE.md).

El piloto 0.1.0 ya está instalado desde cero. Por decisión del usuario se descartaron los datos anteriores; no se ensayó migración conservando datos desde 3.4.0. El MSI es privado y sin firma. Los comandos remotos del candidato se limitan a ping, status y run_diagnostic, con vencimiento; el agente nuevo necesita el servidor nuevo. Producción sigue sin aprobación.

### Estado del piloto limpio — 2026-11-03

Servidor y dashboard nuevos desplegados, 17 migraciones aplicadas y agente Windows 0.1.0 instalado. Recepción de recursos, actividad, ventanas y contadores por sesión, login local y CSV comprobados. Reinicio completo, desconexión prolongada y aceptación multiplataforma siguen pendientes; ver [progreso](docs/REFACTOR_PROGRESS.md).

### Recuperación y publicación del código

Piloto Windows 0.1.0: recuperación automática del servicio en 16 s tras caída; corte RabbitMQ de 75 s con 11 eventos pendientes que luego se entregaron, cola final cero e identidad conservada. No sustituye reinicio completo ni desconexión prolongada. La salud de sesiones se consolida cada cinco minutos, por lo que puede estar vacía en el primer reporte.

Código piloto publicado en `Monitorv3/main` (commit `7217239`): [alcance y credenciales históricas pendientes](docs/GITHUB_PUBLICATION.md). Se retiraron credenciales heredadas del código actual; su revocación/rotación sigue pendiente. Se conserva el historial por decisión del usuario. Esta publicación de código piloto no certifica producción.

Privacidad del escritorio: el candidato incorpora controles en Administración para ocultar títulos/rutas y excluir aplicaciones, aplicados por el agente antes de almacenar y reenviar eventos. Requiere agente actualizado; no desplegado en el piloto. Alcance y límites: [DESKTOP_PRIVACY.md](docs/DESKTOP_PRIVACY.md).

El respaldo programado admite copia externa opcional con rclone y verificación SHA-256 antes de publicar el manifiesto. DevOps configura el remoto y las credenciales fuera del repositorio; no se activó un destino real. Ver [respaldos](docs/ENCRYPTED_BACKUPS.md).

El servidor de producción usa una imagen mínima sin shell/curl y comprobación de salud nativa, fusionada en el [PR #9](https://github.com/Porta0922/Monitorv3/pull/9) el 2026-11-16 tras los cuatro flujos CI aprobados. Escaneo completo del 2026-11-13: cero críticos/altos, 39 medios, 12 bajos y 2 sin severidad determinada; pendientes clasificación y aceptación del conjunto. Desarrollo y piloto no se sustituyen automáticamente. Ver [imágenes](docs/IMAGE_SECURITY.md).
## Estado operativo de alertas (2026-11-17)

El notificador admite webhook HTTPS y correo SMTP con TLS obligatorio. La guía
[de alertas externas](docs/EXTERNAL_ALERTS.md) explica la configuración privada
para DevOps. La entrega real de correo y la aceptación de las imágenes corregidas
siguen pendientes; este avance no habilita producción. El estado vigente y los
bloqueos se mantienen en [TODO](docs/TODO.md).
