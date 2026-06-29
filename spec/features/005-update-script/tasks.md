# 005 · Update script resiliente — Tareas

- [x] Implementar rename antes de copy en `create_update_script()`.
- [x] Agregar timeout de 30s en loop de `sc stop`.
- [x] Agregar segundo `taskkill /F /IM` con sleep de 3s.
- [x] Agregar loop `:wait_proc` hasta que el proceso no exista.
- [x] Guardar y restaurar `sc failure` / `sc failureflag`.
- [x] Validar contra los criterios de aceptación de `spec.md`.
- [x] Mover la feature a "Hecho" en `../../constitution/roadmap.md`.
