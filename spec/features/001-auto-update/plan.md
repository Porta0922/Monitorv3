# 001 · Auto-update — Plan

## Enfoque

Módulo `updater.rs` autocontenido con dos funciones principales: `check_for_update()` (llama a GitHub API) y `download_and_install()` (descarga binario + genera script BAT). El script BAT se ejecuta asíncronamente para manejar el reemplazo del binario en uso.

## Implementación

1. `agent/src/updater.rs` — Módulo completo con lógica de check, download, y script generation.
2. `agent/src/main.rs` — `pub mod updater;` + tarea tokio diaria a las 9:00 AM.
3. `agent/src/ui.rs` — Comando `CMD_CHECK_UPDATE` en el menú contextual.
4. `agent/Cargo.toml` — Agregar `reqwest` con feature `blocking` para consultas HTTP síncronas desde el hilo del tray.

## Decisiones

- **reqwest blocking** — El tray corre en un hilo (no tokio), necesitamos llamadas HTTP síncronas. Usar `reqwest::blocking` evita complejidad.
- **Script BAT externo** — No podemos reemplazar el .exe mientras corre; un script secundario espera, copia, y reinicia.
- **hwnd as usize** — Para pasar el handle de ventana a un hilo `Send`, se convierte a `usize` y luego de vuelta a `HWND`.
- **Rename antes de copy** — Si el .exe está en uso (servicio), `MoveFileEx` lo renombra; luego copiamos el nuevo.
- **API asset URL** — La API devuelve `url` del asset; con `Accept: application/octet-stream` evitamos el redirect CDN que pierde el token.

## Riesgos

- **Archivo bloqueado** — El servicio mantiene el .exe abierto. Mitigado con rename + doble taskkill.
- **Timeout infinito** — Si el servicio no se detiene, el script espera para siempre. Mitigado con timeout de 30s.
- **404 en repos privados** — GitHub API devuelve 404 si no hay token. Mitigado detectando y retornando `UpToDate`.
