# Roadmap

## Hecho ✅

1. **001 · Auto-update vía GitHub Releases** — El agente verifica actualizaciones desde el menú contextual y automáticamente cada 24h a las 9 AM, descarga e instala el nuevo binario con script auto-eliminable.
2. **002 · Instalador PowerShell** — Script PS1 con barra de progreso, detección de config, elevación automática, y settings correctos (encoding ASCII, batería, tarea programada).
3. **003 · Fix encoding XML tarea programada** — Corrección del BOM UTF-8 que causaba error de parsing en `schtasks.exe`.
4. **004 · GITHUB_TOKEN para repos privados** — Autenticación en API de GitHub vía token configurable en `.env` y `agent-config.json`.
5. **005 · Battery-aware scheduled task** — La tarea del agente no se detiene al desconectar corriente en notebooks.
6. **006 · Update script resiliente** — Renombra el .exe en uso antes de copiar, timeout de 30s en stop del servicio, reintentos.

## Siguiente 🔜

*(vacío — resolver issues en producción primero)*

## Backlog / ideas 💡

- **Watchdog externo** — Script o servicio auxiliar que verifique cada N minutos que el agente está vivo.
- **Dashboard web embebido** — Mejorar `localhost:9876` con gráficos y estadísticas en tiempo real.
- **Compilar server** — Arreglar errores de compilación en `activity-monitor-server`.
- **Migrar a edición 2024** — Requiere arreglar `unsafe_op_in_unsafe_fn` y keyword `gen`.
