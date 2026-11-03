# Estructura vigente y retirada del legado

Actualizado: 2026-11-03. Esta limpieza no cambia el protocolo ni la versión 0.1.0 del agente.

## Fuentes activas

- `agent/`: servicio nativo, componente de sesión y vista local.
- `server/`: API, consumidores RabbitMQ y migraciones SQLx en `server/migrations/`.
- `dashboard/`: interfaz React/TypeScript y Nginx.
- `packaging/`: instalación, actualización, desinstalación y gestión de sesiones Windows/Linux.
- `scripts/`: piloto local, mantenimiento, producción y preparación de publicaciones.
- `DockerServer/`, los Compose de raíz y `.github/workflows/`: construcción, despliegue y CI.
- `tools/agent-transport-probe/`, `tests/`: diagnóstico de transporte y validación.
- `shared_types.rs`: contratos compartidos; todavía utilizado por el código.
- `docs/`: documentación operativa y evidencia. `TODO.md` es la lista vigente.

## Retirado del árbol activo

Se eliminaron 73 archivos rastreados, recuperables en el historial Git:

- `Instaladores/`, `instaladorweb/`, `usb/` y el BAT de `USB-Desinstalar/`: vías de instalación antiguas, reemplazadas por `packaging/`.
- `manager/`, `manager.config.json`, `remover/` y `scripts/publish.ps1`: consola/publicador y desinstalador antiguos. La distribución vigente se documenta en [PACKAGE_DISTRIBUTION.md](PACKAGE_DISTRIBUTION.md).
- `scripts/build-release.ps1`, `scripts/build-usb.ps1` y el `Dockerfile.agent` de raíz: construcción obsoleta. Se conservan los Dockerfiles de validación en `tests/`.
- `scratch/` y `server/errors.json`: herramientas transitorias de refactorización y volcado de compilación.
- `diagnostic.ps1`, `limpiar.ps1`, `verify_rabbitmq.ps1`, `scripts_mantenimiento_db.sql`: auxiliares antiguos fuera del flujo vigente. Usar el piloto y mantenimiento documentados en README y [DATA_RETENTION.md](DATA_RETENTION.md).
- `spec/`, `PLAN.md`, `RESUME.md`, `ARCHITECTURE.md`, `API_REFERENCE.md`, `README_SERVER_PRODUCCION.md`, `diagram.png`: especificaciones y guías anteriores incompatibles con el estado actual. Consultar README, [API_AUTHORIZATION.md](API_AUTHORIZATION.md), [TELEMETRY_CONTRACT.md](TELEMETRY_CONTRACT.md) y [PRODUCTION_DEPLOYMENT.md](PRODUCTION_DEPLOYMENT.md).

Cargo y los tres Dockerfiles que copiaban manager/remover fueron ajustados. Cargo.lock solo perdió las entradas de esos dos paquetes; no se actualizaron versiones de dependencias.

## Conservado deliberadamente

- `migrations/` de raíz: esquema legado consumido por las pruebas de migración mediante `LEGACY_SCHEMA_DIR`; no es la fuente de migraciones del servidor actual.
- Informes históricos en `docs/`: evidencia de decisiones y comprobaciones. Sus menciones a rutas eliminadas describen el pasado.
- Compatibilidad dentro del código del agente: requiere evaluar comportamiento antes de retirarla; no se elimina por antigüedad.
- Archivos privados/ignorados, binarios locales, volúmenes Docker e instalación del piloto: no se modificaron. Por eso alguna carpeta antigua puede seguir visible localmente aunque ya no forme parte del proyecto publicado.

La retirada de archivos actuales no borra commits anteriores ni revoca credenciales históricas.

## Comprobación de esta limpieza

`cargo metadata --locked --offline --no-deps`, preflight de publicación y `git diff --check` aprobados. La resolución completa offline se detuvo por falta de `adler2 2.0.1` en caché; no se afirma una compilación completa. No se ejecutó de nuevo la matriz funcional ni se reinstaló el piloto.
