# 004 · Battery-aware scheduled task — Plan

## Enfoque

Después de crear la tarea programada, ejecutar `schtasks /Change` para deshabilitar las opciones de batería.

## Implementación

1. `Instaladores/Windows/post-install.ps1` — Agregar comandos `schtasks /Change` después de la creación de la tarea.
2. `usb/install.ps1` — Ídem.
3. `Instaladores/Windows/install-windows.ps1` — Ídem.

## Decisiones

- `schtasks /Change` sobre modificar el XML manualmente; es más simple y confiable.

## Riesgos

- Mínimo — Windows ignora flags de batería en equipos de escritorio.
