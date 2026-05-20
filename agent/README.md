# ActivityMonitor Agent (Rust)

[![Version](https://img.shields.io/badge/version-3.3.0-blue.svg)](../CHANGELOG.md)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg)]()

*Actualizado: 20 de Mayo, 2026*

El agente de ActivityMonitor es una aplicación ligera y de alto rendimiento desarrollada en Rust diseñada para capturar telemetría detallada de actividad y auditoría de seguridad en endpoints corporativos de manera extremadamente eficiente.

---

## 💎 Capacidades de Captura (Tareas Modulares)

El agente está organizado bajo una estructura modular de tareas independientes en `src/tasks/`:

- **Captura de Ventanas (`app_focus.rs`)**: Monitoreo de la aplicación activa y título de ventana (cada 2s) utilizando APIs nativas del OS.
- **Métricas de Entrada (`inputs.rs`)**: Seguimiento de pulsaciones de teclas, clics y movimiento de mouse capturados con variables atómicas de baja latencia para generar heatmaps horaria.
- **Detección de Inactividad (`activity.rs`)**: Cálculo de tiempo ocioso y mitigación de sobre-reportes tras suspensión del sistema (utiliza Tokio `MissedTickBehavior::Skip` y relojes monotónicos).
- **Inventario de Software (`software.rs`)**: Escaneo periódico y registro de aplicaciones instaladas en el sistema operativo.
- **Monitoreo de USB (`usb.rs`)**: Detección en tiempo real de conexiones/desconexiones de dispositivos de almacenamiento externo y detección de copiado de archivos (DLP Básico) con deduplicación por hashes SHA-256.
- **Redes WiFi (`network.rs`)**: Captura de cambios de conectividad inalámbrica, SSID, BSSID, calidad de señal y estado del adaptador.
- **Seguridad / osquery (`security.rs`)**: Planificador e integrador inteligente con osquery para auditoría de amenazas (11 técnicas MITRE ATT&CK) con perfiles dinámicos centralizados.

---

## 🏛️ Arquitectura y Optimización (v3.3.0)

- **Runtime Asíncrono**: Basado en `tokio` para concurrencia ligera de E/S.
- **Comunicación en Tiempo Real**: Publicación directa sobre colas AMQP mediante RabbitMQ.
- **Caché Offline Reutilizable y Persistente**:
  - Implementa base de datos SQLite cifrada (`agent_offline_cache.db`).
  - Utiliza una conexión persistente thread-safe compartida mediante `Arc<Mutex<Connection>>` en `offline_cache.rs` para eliminar el alto costo de E/S de abrir/cerrar archivos constantemente.
  - Modo **WAL (Write-Ahead Logging)** habilitado con `synchronous = NORMAL` y `temp_store = MEMORY` garantizando persistencia ultrarrápida.
- **Cifrado Vinculado a Hardware (Hardware-Bound)**:
  - La clave de cifrado local se deriva dinámicamente (`resolve_secure_key`) utilizando SHA-256 combinando la clave de entorno, el UUID único del dispositivo y la huella de hardware del sistema operativo:
    - **Windows**: `MachineGuid` de registro.
    - **Linux**: `/var/lib/dbus/machine-id` o `/etc/machine-id`.
    - **macOS**: `IOPlatformUUID` vía IOKit.
  - Previene el descifrado y lectura de la caché offline en cualquier otro host no autorizado.
- **Poda Automática de Logs**: Limpieza automática de logs de depuración (`agent_service.log` y `agent_user.log`) antiguos con antigüedad superior a 7 días en cada arranque.

---

## 💻 Modelos de Ejecución Multiplataforma

### 1) Windows (Sistema Dual)
- **Servicio de Windows (Sesión 0)**: Corre con privilegios de `SYSTEM` encargándose de la persistencia de ejecución, USB, inventario y telemetría de red.
- **Agente de Usuario (Sesión Gráfica)**: Corre en la sesión activa del usuario para capturar interacciones gráficas, títulos de ventana y eventos de entrada que están vedados en Sesión 0.

### 2) macOS (Servicio Dual)
- **LaunchDaemon**: Servicio del sistema ejecutado como `root` para telemetría general y resiliencia.
- **LaunchAgent**: Servicio ejecutado en la sesión de usuario que requiere privilegios de Accesibilidad y Grabación de Pantalla en el panel de TCC.

### 3) Linux (systemd & Wayland)
- Servicio `systemd` que ejecuta de forma optimizada con lógica integrada para determinar el display session activo (Wayland o X11) garantizando la captura de focos de ventanas gráficas.

---

## 🛠️ Requisitos de Compilación e Instalación

### Requisitos
- **Rust (Cargo) 1.75+**
- **Dependencias**: WinAPI en Windows, `libdbus` y dependencias del sistema en Linux, `IOKit` en macOS.

### Compilación Autónoma
```bash
cargo build --release -p activity-monitor-agent
```

### Configuración del Archivo `.env`
El agente lee su configuración local desde un archivo `.env` ubicado en su directorio de trabajo:
- `AGENT_AUTH_TOKEN`: Token seguro de autenticación con el servidor.
- `AGENT_SERVER_URL`: URL del servidor para descarga central de políticas de seguridad.
- `RABBITMQ_URL`: URL de conexión AMQP de RabbitMQ.
- `AGENT_OFFLINE_CACHE_KEY`: Clave secreta base para derivación de cifrado local.
