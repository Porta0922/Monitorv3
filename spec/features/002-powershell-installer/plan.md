# 002 · Instalador PowerShell — Plan

## Enfoque

Dos scripts: `usb/install.ps1` (para instalación USB) e `Instaladores/Windows/install-windows.ps1` (genérico). Ambos comparten la misma lógica pero difieren en paths de origen. Un `post-install.ps1` común maneja la creación de tareas y servicio.

## Implementación

1. `usb/install.ps1` — Instalador USB con `Write-Progress`.
2. `Instaladores/Windows/install-windows.ps1` — Instalador genérico.
3. `Instaladores/Windows/post-install.ps1` — Post-instalación (tarea, servicio, .env).
4. `scripts/build-usb.ps1` — Actualizado para incluir `.ps1` en el build USB.

## Decisiones

- `-Encoding ASCII` en lugar de `utf8` / `UTF-8` para fijar encoding en `out-file`.
- `(Get-Content ...) -replace ...` para escapar `%` en BAT.
- PowerShell y BAT coexisten; no reemplazar el BAT por ahora.

## Riesgos

- **Encoding** — `schtasks.exe` no acepta UTF-8 con BOM. Mitigado con `-Encoding ASCII`.
- **Batería** — La tarea programada por defecto se pausa en batería. Mitigado con `schtasks /Change /DisallowStartIfOnBatteries NO`.
