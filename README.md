# Monitor — Supervisión de equipos de oficina

**Estado: piloto controlado; todavía no aprobado para producción.**

Escaneo reproducible de imágenes y generación de SBOM: [procedimiento](docs/IMAGE_SECURITY.md). Dashboard candidato con base Nginx actualizada y escaneo sin avisos conocidos; los servicios restantes siguen pendientes.

Bloque 1 en curso: clientes Rust actualizados a reqwest 0.12 con JSON/TLS nativo y sin HTTP/2 opcional; eliminado h2 vulnerable y comprobación del workspace Windows aprobada. Esta actualización no está instalada en el piloto; el próximo paquete del agente deberá tener una versión nueva conforme a VERSIONING.md.

Compatibilidad Linux: la enumeración de ventanas pasa a metadatos X11 mediante XCB, sin dependencias de captura de pantalla/PipeWire. Wayland sigue fuera de la captura global admitida. El PR #2 fue fusionado con los cuatro flujos de CI aprobados; queda pendiente la aceptación del agente con escritorios reales.

RabbitMQ 4.2 pasó la validación aislada de identidad y transporte TLS; aún no reemplaza el broker del piloto. Véase el [plan de migración](docs/RABBITMQ_MIGRATION.md).

Plan vigente: [TODO y ruta de cierre hacia producción](docs/TODO.md), reorganizado el 2026-11-06 en ocho bloques con criterios de cierre. Siguiente paso: credenciales y dependencias; después despliegue, datos, agente, paquetes, dashboard y aceptación integrada.

Revisión de seguridad actualizada al **2026-11-06**: [hallazgos, correcciones y pruebas](docs/SECURITY_REVIEW_2026-11-06.md). Incluye endurecimiento local de sesiones, registros, Nginx y configuración. Quedan abiertos el token histórico, dependencias Rust, imágenes Docker y validación del despliegue final. Estos cambios todavía no están desplegados en el piloto.

**Agente piloto instalado: 0.1.0, sin publicación estable.** Actualizado: 2026-11-03.

Monitor reúne recursos, actividad de aplicaciones, inventario y eventos de computadoras de oficina. El agente se instala en cada equipo; el servidor central recibe la información y el dashboard permite consultarla. RabbitMQ se mantiene como transporte de eventos.

La nueva numeración comienza en 0.1.0. El piloto local fue reinstalado desde cero por autorización del usuario, descartando los datos 3.4.0. El MSI nuevo es privado y sin firma; los artefactos antiguos 3.x no deben reutilizarse. Esto no valida la migración que conserva datos de otras instalaciones. Consulta [versionado](docs/VERSIONING.md).

## Arquitectura

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
| Redis | Incluido en Compose; uso funcional por confirmar | No tratar aún como requisito de la aplicación |

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
| Redis | 127.0.0.1:63790 |

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

Existe un [overlay de producción en preparación](docs/PRODUCTION_DEPLOYMENT.md), separado del piloto: roles PostgreSQL separados, trabajo de migraciones independiente, servidor/dashboard sin root, HTTPS en loopback 18443 y AMQPS en loopback 25671. Usa secretos desde archivos y validación estricta al arrancar. Redis queda excluido por defecto. El generador requiere Linux, cuentas Argon2id y certificados válidos; ver la guía para nombres, CA y permisos.

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
