# Changelog - ActivityMonitor Enterprise v3

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
