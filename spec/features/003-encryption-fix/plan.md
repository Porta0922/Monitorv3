# 003 · Fix encoding XML — Plan

## Enfoque

Cambiar `-Encoding UTF8`/`UTF-16` a `-Encoding ASCII` en los comandos `Out-File` y `Set-Content` que generan los XML de tareas programadas.

## Implementación

1. `usb/install.ps1` — Cambiar encoding.
2. `Instaladores/Windows/install-windows.ps1` — Cambiar encoding.
3. `Instaladores/Windows/post-install.ps1` — Cambiar encoding.

## Decisiones

- `ASCII` porque PowerShell en Windows escribe UTF-8 sin BOM con ese flag.
- Aplica a los 3 instaladores.

## Riesgos

- Bajo — El cambio es directo y probado.
