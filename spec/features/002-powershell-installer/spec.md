# 002 · Instalador PowerShell

**Estado:** implementado ✅

## Qué hace

Provee scripts de instalación en PowerShell (`.ps1`) como alternativa a los `.bat` heredados. Muestran barra de progreso, detectan si el agente ya está instalado, permiten desinstalación, copian configuración desde archivo JSON o USB, y manejan auto-elevación.

## Por qué

Los `.bat` son frágiles, difíciles de leer, no muestran progreso, y se cierran sin avisar. PowerShell ofrece mejor manejo de errores, salida formateada, y experiencia de usuario.

## Criterios de aceptación

- [x] Barra de progreso `Write-Progress`.
- [x] Detección de instalación existente.
- [x] Copia de `agent-config.json` y `.env` a `C:\ProgramData\ActivityMonitor\`.
- [x] Corrección de encoding XML (UTF-8 sin BOM para `schtasks`).
- [x] Forzar `DisallowStartIfOnBatteries=false` en tarea programada.
- [x] Auto-elevación si no es admin.
- [x] Escribir `GITHUB_TOKEN` en `.env`.

## Fuera de alcance

- Interfaz gráfica (WPF/WinForms).
