# ActivityMonitor Agent (Rust)
*Actualizado: 28 de Abril, 2026*

El agente de ActivityMonitor es una aplicación ligera desarrollada en Rust diseñada para capturar telemetría de actividad y seguridad en endpoints.

## Capacidades

- **Captura de Ventanas**: Monitoreo de la aplicación activa y título de ventana (cada 2s).
- **Métricas de Entrada**: Seguimiento de pulsaciones de teclas, clics y movimiento de mouse para generar heatmaps de intensidad.
- **Detección de Inactividad**: Cálculo de tiempo ocioso basado en la actividad del usuario.
- **Inventario de Software**: Escaneo de aplicaciones instaladas (inicial + semanal).
- **Monitoreo de USB**: Registro de conexiones/desconexiones y detección de copiado de archivos (DLP básico).
- **Redes WiFi**: Registro de cambios en la conexión WiFi (SSID, señal, estado).
- **Seguridad (osquery)**: Integración con osquery para detección de amenazas MITRE ATT&CK.

## Arquitectura

- **Async Runtime**: Basado en `tokio`.
- **Comunicación**: Protocolo AMQP mediante RabbitMQ para envío de eventos.
- **Resiliencia**: Cache offline en SQLite (`agent_offline_cache.db`) para almacenar eventos cuando no hay conexión.
- **Multiproceso (Windows)**: 
  - **Servicio de Windows**: Ejecuta como SYSTEM para tareas de bajo nivel y persistencia.
  - **Agente de Usuario**: Ejecuta en la sesión del usuario para capturar actividad interactiva (ventanas, mouse, teclado).

## Despliegue en Windows

El agente soporta tres modos de instalación mediante `deploy/install-windows.bat`:

1. **Instalación Completa (Híbrida)**: Configura el servicio y la tarea de usuario. Recomendado para monitoreo total.
2. **Solo Servicio**: Ideal para servidores o monitoreo de seguridad en segundo plano.
3. **Solo Usuario**: Para estaciones de trabajo donde no se requieren privilegios administrativos de servicio.

## Requisitos de Compilación

- Rust (Cargo) 1.75+
- Dependencias de sistema (WinAPI en Windows, libdbus en Linux).

```bash
cargo build --release -p activity-monitor-agent
```

## Configuración (.env)

El agente puede configurarse mediante variables de entorno o un archivo `.env` en su directorio:

- `AGENT_AUTH_TOKEN`: Token de autenticación con el servidor.
- `AGENT_SERVER_URL`: URL del servidor para descarga de políticas.
- `RABBITMQ_URL`: URL de conexión a la cola de mensajes.
- `AGENT_OFFLINE_CACHE_KEY`: Clave para cifrado/ofuscación de la cache local.
