# 004 · Battery-aware scheduled task

**Estado:** implementado ✅

## Qué hace

La tarea programada del agente (`ActivityMonitorUserAgent`) no se detiene cuando el equipo notebook se desconecta de la corriente eléctrica. Corre siempre, independientemente del estado de batería.

## Por qué

Por defecto Windows crea tareas programadas con `DisallowStartIfOnBatteries=true` y `StopIfGoingOnBatteries=true`. El agente de monitoreo debe correr 24/7 incluso en notebooks sin corriente.

## Criterios de aceptación

- [x] Después de instalar, la tarea tiene `DisallowStartIfOnBatteries=false`.
- [x] Después de instalar, la tarea tiene `StopIfGoingOnBatteries=false`.
- [x] Se aplica con `schtasks /Change`.

## Fuera de alcance

- Optimización de consumo energético (el agente corre siempre).
