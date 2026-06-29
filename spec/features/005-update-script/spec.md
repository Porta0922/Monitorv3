# 005 · Update script resiliente

**Estado:** implementado ✅

## Qué hace

El script BAT de actualización (`am_update.bat`) maneja correctamente archivos bloqueados: renombra el `.exe` en ejecución antes de copiar el nuevo, tiene timeout de 30s para detener el servicio, fuerza kill con reintentos, y restaura los recovery options del servicio después de la actualización.

## Por qué

El update fallaba cuando el binario estaba en uso por el servicio de Windows (error "The process cannot access the file because it is being used by another process"). Además, si el servicio no se detenía rápidamente, el script esperaba indefinidamente.

## Criterios de aceptación

- [x] El script renombra `activity-monitor-agent.exe` a `activity-monitor-agent.exe.old` antes de copiar.
- [x] Timeout de 30s en `sc stop`; si no se detiene, `taskkill /F`.
- [x] Doble `taskkill /F /IM` por si algún proceso revive entre el primer y segundo kill.
- [x] Loop de espera hasta que el proceso ya no exista (`goto wait_proc`).
- [x] Restaurar `failure` y `failureflag` del servicio después del update.
- [x] El script se auto-elimina al finalizar.

## Fuera de alcance

- Verificación checksum del binario descargado.
- Rollback si falla la copia.
