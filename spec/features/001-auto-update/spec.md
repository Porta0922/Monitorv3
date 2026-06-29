# 001 · Auto-update vía GitHub Releases

**Estado:** implementado ✅

## Qué hace

El agente puede actualizarse a sí mismo desde el menú contextual ("Buscar actualizaciones") y automáticamente cada día a las 9:00 AM. Descarga el binario `activity-monitor-agent.exe` desde la última Release de GitHub, detiene el servicio y la tarea programada, reemplaza el binario, y reinicia ambos.

## Por qué

No hay un mecanismo de actualización centralizada (GPO, MDM, etc.). El agente necesita poder actualizarse autónomamente sin intervención del usuario ni scripts externos.

## Criterios de aceptación

- [x] "Buscar actualizaciones" en el menú contextual del tray verifica la última versión en GitHub.
- [x] Si hay nueva versión, muestra un cuadro de confirmación antes de descargar.
- [x] La descarga usa `Accept: application/octet-stream` contra la API de GitHub (funciona con repos privados).
- [x] El script de actualización (`.bat`) se crea en `%TEMP%\am_update.bat` y se auto-elimina.
- [x] El update renombra el `.exe` en uso antes de copiar el nuevo (evita "file in use").
- [x] El servicio se detiene con timeout de 30s; si no responde, se fuerza kill.
- [x] La tarea programada se reinicia después del update.
- [x] Los recovery options del servicio se restauran después del update.
- [x] Auto-update diario a las 9:00 AM sin interacción del usuario.
- [x] Sin `MessageBox` bloqueante durante el auto-update automático.
- [x] Usa `GITHUB_TOKEN` del `.env` para autenticación en repos privados.

## Fuera de alcance

- Firmado de binarios (no se requiere).
- Canal estable/canary (solo un canal).
