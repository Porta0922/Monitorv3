# Tech stack y convenciones

## Tecnologías

- **Lenguaje:** Rust 2021 edition (minimum 1.75+)
- **Framework / runtime:** Tokio (async), reqwest (HTTP), lapin (RabbitMQ), axum (server)
- **Base de datos:** SQLite vía rusqlite con cifrado AES-256-GCM (offline cache)
- **Tests:** No hay suite formal — se valida con `cargo build` + pruebas manuales
- **Despliegue:** GitHub Actions → Release + USB installer scripts

## Archivos / módulos clave

- `agent/src/main.rs` — Punto de entrada, inicialización, `run_agent()` con supervisor de tareas.
- `agent/src/ui.rs` — Bandeja de sistema (icono, menú contextual, actualización).
- `agent/src/updater.rs` — Auto-update vía GitHub Releases.
- `agent/src/discovery.rs` — Descubrimiento de configuración (JSON, registro, env).
- `agent/src/rabbitmq_publisher.rs` — Publicación de eventos en RabbitMQ.
- `agent/src/offline_cache.rs` — Caché offline cifrada en SQLite.
- `agent/src/task_supervisor.rs` — Supervisor que reinicia tareas internas si fallan.
- `agent/src/monitoring.rs` — Captura de ventana activa (Windows).
- `agent/src/keystroke_tracker.rs` — Conteo de pulsaciones + detección de idle.
- `agent/src/input_tracking.rs` — Heatmap de input (teclado/ratón).
- `agent/src/usb_detection.rs` / `usb_file_copy_detection.rs` — Detección de dispositivos USB.
- `agent/src/wifi_detection.rs` — Escaneo de redes WiFi.
- `agent/src/process_protection.rs` — Anti-terminación vía Job Object.
- `agent/src/web.rs` — Servidor HTTP local para estado (localhost:9876).
- `agent/src/command_channel.rs` — Canal de comandos remotos vía RabbitMQ.
- `agent/src/health_reporter.rs` — Reporte de salud del agente.
- `server/src/` — API REST con axum (actualmente con errores de compilación).
- `Instaladores/` / `usb/` — Scripts de instalación (PS1 + BAT).
- `.github/workflows/build-release.yml` — CI/CD.

## Comandos

- `cargo build --release -p activity-monitor-agent` — Compila solo el agente.
- `cargo check -p activity-monitor-agent` — Verifica compilación sin generar binario.
- `cargo build --release` (workspace) — Compila todo (falla si server tiene errores).
- `scripts/build-usb.ps1` — Genera los archivos para instalación USB.

## Modelo de datos / dominio

- **Evento** — Unidad básica de telemetría. Tipos: `window_activity`, `heartbeat`, `usb_detected`, `app_running`, `wifi_network`, `keystroke_summary`, `heatmap`, `inventory`, `security_event`.
- **Device** — Identidad única del equipo (`device_id` UUID v4, hostname, MAC address).
- **OfflineCache** — SQLite cifrada con tabla única `events` (payload JSON + timestamp + metadata).
- **RabbitMQ** — Exchange `activity-monitor`, routing key por tipo de evento.

## Convenciones

- `snake_case` para variables y funciones en Rust.
- `camelCase` para campos en JSON y config (`agent-config.json`).
- Todo el contenido del agente en español (logs, UI, instaladores). Código fuente en inglés.
- Manejo de errores con `tracing::error!` + `Result`; pánicos capturados con `register_panic_hook()`.
- `.env` en `C:\ProgramData\ActivityMonitor\.env`; configuración adicional en `agent-config.json`.
- Sin dependencias externas sin revisión; prefierir `winapi` directo sobre wrappers.

## Límites duros

- No commitear tokens, claves, o `.env` al repositorio.
- No usar `cargo build --release` sin `-p activity-monitor-agent` (el server no compila).
- No modificar `server/` sin coordinar (tiene errores preexistentes no relacionados).
- El agente solo corre en Windows (target `cfg(windows)` en varias secciones).
