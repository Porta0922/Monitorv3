# Changelog - ActivityMonitor Enterprise v3

## [3.3.0] - 2026-05-20
### Añadido
- **Cifrado Vinculado al Hardware (Hardware-Bound Encryption)**: Implementado enlace de cifrado dinámico y robusto en la base de datos `OfflineCache` del agente. La clave de encriptación se deriva a través de SHA256 combinando la clave del entorno, el UUID estable del dispositivo (`device_id`) y el identificador de máquina nativo del sistema operativo (MachineGuid en Windows, machine-id en Linux y IOPlatformUUID en macOS), previniendo el descifrado no autorizado en otros hosts.
- **Poda y Rotación de Logs de Telemetría**: Implementada lógica automática de limpieza de registros para evitar que los archivos de log (`agent_service.log` y `agent_user.log`) acumulen espacio de manera indefinida, purgando automáticamente los archivos históricos con antigüedad mayor a 7 días.

### Corregido
- **Rendimiento de SQLite (Persistent WAL Connection)**: Refactorizado `offline_cache.rs` para reutilizar una conexión única y persistente thread-safe protegida por un `Mutex`, eliminando el costo de E/S de abrir y cerrar el archivo de base de datos en cada evento. Adicionalmente se habilitó el modo Write-Ahead Logging (WAL) junto con temp_store en MEMORY y synchronous NORMAL, multiplicando la velocidad de persistencia.

## [3.2.6] - 2026-05-19
### Añadido
- **Detección de Suspensión del Sistema**: Implementada lógica de detección de suspensión/bloqueo de la computadora en el bucle principal de actividad.

### Corregido
- **Estabilidad post-suspensión (Tokio Missed Ticks)**: Modificados todos los bucles de `tokio::time::interval` (12 en total) para usar `MissedTickBehavior::Skip` en lugar del comportamiento por defecto `Burst`. Esto evita la sobrecarga extrema de CPU y de solicitudes a RabbitMQ al despertar de la suspensión.
- **Inflación de Duración de Actividad**: Resuelto el problema donde el tiempo en suspensión se reportaba como activo para la última ventana enfocada (ej. `lockapp.exe` durante 3 horas), finalizando la duración de la ventana con la última hora activa real pre-suspensión.
- **Métricas del Resumen de Entrada e Inactividad**: Corregida la acumulación desmedida de inactividad acumulada durante la suspensión del sistema en las métricas de `keystroke_tracker` y en los reportes de resumen de entrada.
- **Prueba unitaria de Caché fuera de línea**: Corregido test de encriptación de base de datos en `offline_cache.rs` para usar un archivo temporal en lugar de `:memory:` garantizando persistencia en conexiones cerradas.

## [3.2.5] - 2026-05-13
### Añadido
- **Resiliencia del Agente (Windows)**:
    - Implementado un "Watchdog" de 60 minutos en la tarea programada (`/RI 60`) para asegurar el reinicio automático tras fallos o suspensiones del sistema.
    - Añadido retraso de inicialización (3s) en modo interactivo para garantizar que el escritorio de Windows esté listo antes de capturar actividad.
- **Robustez de Ejecución**:
    - Reemplazados `unwrap()` críticos por manejo de errores seguro y logging detallado durante el arranque.
    - Mejorada la detección de errores en la creación del runtime de Tokio.

### Corregido
- **Fallo de Detección de Actividad**: Resuelto el problema donde el agente de usuario se detenía silenciosamente tras errores de sesión, restaurando la captura de ventanas y eventos de entrada.

## [3.2.4] - 2026-05-05
### Añadido
- **Mejoras en el Despliegue USB**:
    - Creada una guía de instalación de Linux paso a paso completa (`Guia_Instalacion_Linux.txt`) en español.
    - Organizada la estructura del USB con rutas relativas optimizadas para asegurar que los instaladores funcionen en diferentes entornos.
- **Instalador de Linux Mejorado**:
    - Refactorizado `install-linux.sh` para manejar correctamente la compilación independiente desde el USB.
    - Añadida detección e instalación automática de Rust y dependencias del sistema en Linux.
    - Asegurado que los scripts de instalación de Linux usen terminaciones de línea Unix (LF) para eliminar el requisito de `dos2unix`.
- **Soporte de Agente Independiente**:
    - Desvinculado `agent/Cargo.toml` del workspace raíz para permitir la compilación en máquinas sin el código fuente completo del repositorio.

### Corregido
- **Error de Compilación en Linux**: Resuelto el error "failed to find workspace root" durante la compilación del agente en Linux.
- **Limpieza de USB**: Eliminados archivos innecesarios del proyecto (código del servidor, cachés locales, artefactos de compilación redundantes) del USB de despliegue para reducir el tamaño y la confusión.

## [3.2.3] - 2026-05-04
### Fixed
- Resolved activity over-reporting issue (e.g., Chrome usage exceeding real time).
- Fixed single-instance mutex logic to correctly handle non-admin sessions and prevent multiple agents from running simultaneously.
- Eliminated "ghost time" accumulation when no window is in focus.
- Added detailed activity duration logging for easier auditing.

## [3.2.2] - 2026-05-04

### Corregido
- **Reporte de Actividad Excesivo**: 
    - Migrado el cálculo de duraciones a relojes monotónicos (`Instant`), eliminando discrepancias causadas por ajustes del reloj del sistema (NTP/Sincronización).
    - Refactorizado el bucle de monitoreo para asegurar segmentos de tiempo estrictamente no solapados entre cambios de ventana y heartbeats periódicos.
    - Corregido el reporte de "Segundos Activos" en el resumen de entrada para reflejar el tiempo real transcurrido mediante medición de intervalos exactos.

## [3.2.1] - 2026-04-29

### Corregido
- **Optimización Crítica de CPU**: 
    - Refactorizado el sistema de rastreo de entrada para usar **Atómicos**, eliminando la creación de cientos de tareas de Tokio por segundo durante el movimiento del mouse.
    - Implementada verificación de "Delta de Espacio Libre" en USB; el escaneo recursivo solo se activa si el espacio libre en la unidad ha cambiado.
    - Eliminado el doble cálculo de hash de ejecutables en la captura de aplicaciones abiertas.
    - Sincronizados y espaciados los intervalos de tareas en segundo plano (Heartbeat y USB ahora cada 60s).
    - **Seguridad (osquery)**: El escaneo de seguridad ahora se ejecuta **únicamente cuando el usuario está en idle**. Se eliminó la consulta de validación de firmas digitales (`authenticode`) que causaba picos del 40% de CPU.
- **Doble Actividad**: Añadida protección de instancia única mediante Mutex de Windows para evitar que el agente se ejecute múltiples veces en la misma sesión.
- **Inconsistencia de Datos**: Refinamiento de roles híbridos; el servicio (Sesión 0) ahora omite tareas de UI y el agente de usuario omite tareas de inventario/sistema.
- **Rendimiento de Ventanas**: Optimizada la lógica de captura de aplicaciones abiertas para procesar hashes una sola vez por ejecutable único.
- **Detección de USB**: Refinada la lógica de identificación de dispositivos usando `Win32_DiskDrive` y `USBSTOR` para mayor precisión.

- **Detección de Copiado**: Mejorada la sensibilidad de captura comparando `CreationTime` y `LastWriteTime` para detectar archivos movidos o copiados recientemente.
- **Powershell Silent Mode**: Implementado `CREATE_NO_WINDOW` para todas las llamadas a PowerShell relacionadas con USB, evitando parpadeos de consola.

## [3.2.0] - 2026-04-28

### Añadido
- **Arquitectura Híbrida (Windows)**: Implementación de sistema dual con Servicio de Windows (SYSTEM) para persistencia y Agente de Usuario para captura de actividad interactiva.
- **Detección de Sesión 0**: Lógica interna para detectar ejecución en servicios y derivar la captura de ventanas al agente de usuario.
- **Bases de Datos Separadas**: Implementación de `agent_service_cache.db` y `agent_user_cache.db` para evitar bloqueos de archivos en ejecuciones concurrentes.
- **Instalador Multi-Plataforma**: Preparación del USB con código fuente y scripts para despliegue en Linux (`systemd`) y macOS.
- **Documentación Completa**: Actualización de `README.md`, `ARCHITECTURE.md`, `API_REFERENCE.md` y creación de `agent/README.md`.

### Corregido
- **Error de Acceso Denegado**: El instalador ahora otorga permisos de "Modificación" al grupo de Usuarios en `C:\ProgramData\ActivityMonitor` para permitir el registro de logs del agente de usuario.
- **Rutas Absolutas**: Corrección de rutas relativas que causaban que la base de datos se creara en `System32` al ejecutar como servicio.
- **Errores de Sintaxis en Batch**: Eliminado el error "No se esperaba [SKIP]" mediante el escape correcto de caracteres en bloques condicionales.
- **Variables Indefinidas**: Corregido fallo en la aplicación de permisos por variable `INSTALL_DIR` no declarada.
- **Estabilidad de Compilación**: Resueltos errores de propiedad (move) y advertencias de variables no usadas en el código del agente.

## [3.1.0] - 2026-04-24

### Añadido
- Soporte para detección de copiado de archivos a USB (DLP Básico).
- Integración con osquery para alertas de seguridad MITRE ATT&CK.
- Heatmaps de actividad de teclado y mouse.
- Migración a TimescaleDB para almacenamiento optimizado de series temporales.

## [3.0.0] - 2026-04-20
- Lanzamiento inicial de la versión Enterprise v3.
- Arquitectura distribuida con RabbitMQ y Backend en Rust.
- Dashboard en React con soporte para múltiples dispositivos.
