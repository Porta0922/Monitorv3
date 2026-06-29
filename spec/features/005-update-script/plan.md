# 005 · Update script resiliente — Plan

## Enfoque

Modificar el script BAT generado por `updater.rs` para que use `move` (rename) en lugar de `copy` sobre el archivo en uso, timeout en `sc stop`, y recuperación de recovery options.

## Implementación

1. `agent/src/updater.rs` — Modificar `create_update_script()`.

## Decisiones

- **Rename** — Windows permite renombrar un archivo abierto por otro proceso (cambio de entrada de directorio, no de data).
- **30s timeout** — Suficiente para que el servicio termine limpiamente; si no, force kill.
- **Doble force kill** — Un solo `taskkill /F` ocasionalmente falla porque el proceso se reinicia entre el kill y el check. Un segundo kill 3s después lo atrapa.
- **SC failure restore** — `sc failure` se resetea al hacer `sc stop`/`sc start`; hay que re-aplicarlo con `sc failure` y `sc failureflag`.

## Riesgos

- **Loop infinito** — Si el proceso nunca muere, el script se queda en `goto wait_proc`. El doble force kill lo minimiza.
