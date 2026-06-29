# Misión

## Qué construimos

ActivityMonitor es un agente de monitoreo y seguridad para entornos empresariales. Captura actividad del usuario (ventanas activas, aplicaciones en ejecución, dispositivos USB, conectividad WiFi, pulsaciones de teclado) y la envía en tiempo real a un servidor central vía RabbitMQ para su análisis y auditoría.

1. **activity-monitor-agent** — Agente cliente que se ejecuta como servicio de Windows y tarea programada en cada equipo. Captura telemetría, la cachea offline, y la publica en RabbitMQ.
2. **activity-monitor-server** — Servidor central que recibe, procesa y expone los eventos vía API REST (axum).
3. **Instaladores USB** — Scripts PS1/BAT para desplegar el agente en equipos sin conexión a internet corporativa.

## Para quién

- **Administradores de sistemas / TI** — Necesitan visibilidad de qué hacen los usuarios en sus equipos.
- **Auditores de seguridad** — Requieren registros de actividad, detección de dispositivos USB, y trazabilidad.
- **Soporte técnico** — Diagnosticar problemas de conectividad y comportamiento de equipos remotos.

## Principios

- **Offline-first** — El agente nunca pierde datos: cachea en SQLite cifrada y reenvía cuando recupera conexión.
- **No intrusivo** — El agente corre en background sin molestar al usuario. Sin notificaciones, sin ventanas.
- **Resiliente** — Se auto-recupera ante cortes de red, reinicios, y crashes con reintentos exponenciales.
- **Seguro por defecto** — Todo el tráfico cifrado, claves únicas por equipo, autenticación por token.

## Qué NO es

- **Un keylogger malicioso** — Solo cuenta pulsaciones, no registra contenido ni captura contraseñas.
- **Un RAT** — No permite control remoto del equipo, solo monitoreo pasivo.
- **Un producto SaaS** — Es una solución on-premise con servidor propio.
