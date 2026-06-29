# 001 · Auto-update — Tareas

- [x] Leer `GITHUB_TOKEN` desde `.env` en `discovery.rs`.
- [x] Implementar `check_for_update()` con `reqwest::blocking`.
- [x] Implementar `download_and_install()` que descarga y genera script BAT.
- [x] Manejar 404 de GitHub API como `UpToDate`.
- [x] Agregar `pub mod updater;` en `main.rs`.
- [x] Implementar tarea tokio diaria a las 9:00 AM.
- [x] Agregar `CMD_CHECK_UPDATE` en `ui.rs` con hilo separado.
- [x] Remover `MessageBox` bloqueante del auto-update automático.
- [x] Usar `Accept: application/octet-stream` en descarga.
- [x] Renombrar .exe viejo a .old antes de copiar nuevo.
- [x] Timeout de 30s en stop del servicio.
- [x] Restaurar recovery options del servicio después del update.
- [x] Validar contra los criterios de aceptación de `spec.md`.
- [x] Mover la feature a "Hecho" en `../../constitution/roadmap.md`.
