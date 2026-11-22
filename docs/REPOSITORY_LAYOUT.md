# Estructura vigente y retirada del legado

Actualizado: 2026-11-21. Limpieza de archivos; no modifica la versión 0.2.3
ni el comportamiento del agente instalado.

## Limpieza local del 21/11

Retirados aproximadamente **25,96 GiB / 26.936 archivos generados**:

- `target/` (compilaciones y fixtures temporales) y `dashboard/dist/`.
- Dos cachés de Trivy; conservados los informes de vulnerabilidades y SBOM.
- Copias temporales de publicación GitHub y UX (`publish*`, `github-upload`,
  `ux-publication` dentro de `.local-test`).
- Paquetes/candidatos 0.1.0 y 0.2.0, instalador inicial y ZIP del piloto antiguo.
- Binarios, paquetes ZIP/MSI/DEB y símbolos de la entrega 0.2.1; conservados
  manifiestos, hashes y recibos pequeños como evidencia de procedencia.
- Dos TODO históricos del árbol de documentación y configuración vacía de VS Code;
  recuperables en Git. [TODO.md](TODO.md) es la única lista vigente.

Comprobados los destinos dentro del workspace y ausencia de enlaces antes del
borrado recursivo. No se borraron volúmenes Docker, configuración privada ni datos
del agente instalado en ProgramData. No hay procesos activos del piloto ejecutados
desde las carpetas eliminadas. Registro privado: `.local-test/cleanup-20261121.json`.

Se conservan `.tools/`, `.wix/` y `dashboard/node_modules/` como herramientas y
dependencias de desarrollo útiles; `.local-test/.env`, `agent.env`, evidencias de
seguridad/aceptación, la entrega anterior 0.2.2 para soporte y los recibos actuales
0.2.3. Los artefactos publicados de 0.2.3 siguen disponibles en GitHub.

La próxima compilación Rust reconstruirá `target/`; el build frontend recreará
`dist/`. Trivy volverá a descargar su base de avisos cuando se ejecute.

## Fuentes activas

- `agent/`: servicio nativo, componente de sesión y vista local.
- `server/`: API, consumidores RabbitMQ y migraciones SQLx en `server/migrations/`.
- `dashboard/`: interfaz React/TypeScript y Nginx.
- `packaging/`: instalación, actualización, desinstalación y gestión de sesiones Windows/Linux.
- `scripts/`: piloto local, mantenimiento, producción y preparación de publicaciones.
- `DockerServer/`, `DockerPostgres/`, los Compose de raíz y `.github/workflows/`: construcción, despliegue y CI. La imagen de PostgreSQL endurecida sigue siendo candidata hasta completar su aceptación.
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
- Informes de validación en `docs/`: evidencia útil para pendientes abiertos. Sus menciones a rutas eliminadas describen el pasado; los TODO históricos se consultan en Git.
- Compatibilidad dentro del código del agente: requiere evaluar comportamiento antes de retirarla; no se elimina por antigüedad.
- Configuración privada, evidencias, volúmenes Docker e instalación del piloto: preservados. Los binarios antiguos y cachés enumerados arriba sí se retiraron.

La retirada de archivos actuales no borra commits anteriores ni revoca credenciales históricas.

## Comprobación de la limpieza anterior

`cargo metadata --locked --offline --no-deps`, preflight de publicación y `git diff --check` aprobados. La resolución completa offline se detuvo por falta de `adler2 2.0.1` en caché; no se afirma una compilación completa. No se ejecutó de nuevo la matriz funcional ni se reinstaló el piloto.
