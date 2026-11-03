# Ciclo de vida del agente — P2

Estado: implementación parcial actualizada al 2026-10-27. Compilación y pruebas
focalizadas aprobadas; instalación real SCM/systemd pendiente. Evidencia en
P4_VALIDATION.md y operación administrativa en PLATFORM_LIFECYCLE.md.

## Roles y capacidades

AGENT_MODE admite service, interactive o session. Sin valor conserva detección heredada
(Session 0 en Windows, root en Unix). El modo explícito permite ejecutar systemd
con una cuenta dedicada sin confundirlo con una sesión de escritorio.

El servicio conserva telemetría del sistema y transporte RabbitMQ; no instala hooks
ni arranca window_activity/heatmap. En Linux Wayland y sin DISPLAY esos recolectores
se omiten también. X11 y Windows son elegibles, pero la inicialización del listener
puede fallar: input_available describe su resultado. Los contadores heredados siguen
siendo numéricos por compatibilidad, acompañados de ese indicador; no se acumula
actividad de usuario cuando la entrada no está disponible. Falta persistir y mostrar
este indicador en servidor y dashboard.

El modo interactive conserva el agente heredado completo. Ahora existe un modo
session separado que envía solamente eventos de escritorio al servicio por IPC local
cifrado. No instalar interactive junto con service+session. La implementación y sus
límites de un usuario están en SESSION_COMPONENT.md; validación pendiente en P4.

## Estado persistente

- AGENT_CONFIG_FILE selecciona el archivo dotenv; variables del proceso prevalecen.
- AGENT_DATA_DIR selecciona la carpeta de identidad y caché SQLite.
- AGENT_LOG_DIR selecciona la carpeta de logs diarios.
- Windows conserva ProgramData/ActivityMonitor como base predeterminada.
- Linux servicio usa /var/lib/activity-monitor; interactivo usa XDG_DATA_HOME o HOME.

La caché no cae silenciosamente a memoria. Errores de acceso detienen el arranque;
una identidad existente corrupta tampoco se reemplaza. No cambiar la clave de caché
ni borrar device_id.json al actualizar. Escritura atómica de identidad, coordinación
entre procesos y migración de permisos aún pendientes.

## Servicio y recuperación

packaging/linux/activity-monitor-agent.service usa una cuenta sin login, estado/logs
0700, reinicio on-failure tras 15 segundos y máximo 5 arranques en 300 segundos.
Stop administrativo no dispara recuperación. El agente recibe SIGTERM y SIGINT.
La cuenta restringida puede limitar inventario/osquery/USB; se declararán esas
capacidades según su disponibilidad, sin elevar todos los recolectores a root.

packaging/linux/install-service.sh recibe binario ya construido y archivo dotenv.
Solo admite instalación inicial: bloquea servicios o identidades existentes y no
sobrescribe configuración. Habilita el servicio para boot pero lo deja detenido.
No es todavía un DEB ni un mecanismo de actualización.

packaging/windows/register-service.ps1 registra un binario ya desplegado y configura
su archivo dotenv por servicio. Requiere carpetas controladas por administradores.
Configura dos reinicios (15 y 60 segundos), después ninguna acción, reiniciando el
contador tras 24 horas. Deja el servicio detenido. La ACL y distribución del binario
serán responsabilidad del MSI pendiente. Stop/Shutdown solicitan cierre sin bloquear
el callback; un error de run_agent se comunica con código de salida distinto de cero.

Se retiró la activación de anti-kill y del reinicio WER en main. El gestor de servicios
es responsable de recuperación; la parada administrativa ya no genera por sí sola
una alerta de ataque. Los módulos heredados permanecen hasta completar la migración.

## Trabajo restante

Completar enrolamiento/permisos/políticas del componente de sesión, cierre de tareas y persistir
políticas, agregar adaptadores de disco/red/uptime, reemplazar actualizador y
registradores heredados por MSI/DEB con firma, rollback y desinstalación explícita.
No se han ejecutado los scripts nuevos ni se han modificado servicios del equipo.


## Validación focalizada del ciclo de vida

En Windows, compilar primero con `cargo test --locked -p activity-monitor-agent
--no-run`. Ejecutar por separado los filtros `lifecycle_tests`, `policy_tests`,
`durability_tests`, `ipc_tests` y `persistence_tests` con el mismo comando
(reemplazando `--no-run`). En Unix añadir `runtime_signal_tests`.

Los contenedores reproducibles usan:

```powershell
docker build -f tests/Dockerfile.agent -t monitor-agent-debian-validation .
docker build -f tests/Dockerfile.agent-ubuntu -t monitor-agent-ubuntu-validation .
```

Estas pruebas no arrancan recolectores de escritorio ni instalan servicios.
Validan cancelación del supervisor, cambios de política, integridad de cola y
snapshots, rechazo de IPC inválido y recepción de SIGTERM en Unix. No sustituyen
pruebas del servicio instalado, hooks nativos, escritorio o sesiones múltiples.
El apagado espera los wrappers supervisados, que cancelan sus hijos al liberarse;
esto no garantiza que un hilo nativo bloqueado haya finalizado.

## Cierre web y componentes nativos — 2026-10-26

La web local usa conexiones HTTP asíncronas propiedad del supervisor. Al cancelar
el listener se cancelan también las conexiones, incluso peticiones incompletas.
Escucha en 127.0.0.1:9876 por defecto; WEB_BIND permite configuración explícita.
API requiere token exacto; cuerpo máximo 16 KiB, lectura hasta 5 s, conexión hasta
30 s y máximo 32 conexiones simultáneas. WEB_UI_TOKEN configura el token local.

Los hooks y bandeja Windows cooperan con una señal de cierre; los bucles nativos
procesan mensajes sin bloquear indefinidamente en GetMessage. Se espera hasta 2 s
por hilo y se registra si no termina. Menús/diálogos modales todavía requieren
ensayo de escritorio real. El listener rdev Linux/macOS no tiene cancelación
cooperativa implementada; la finalización del proceso sigue siendo su límite.

El ensayo `python tests/agent_runtime.py --image monitor-agent-debian-validation`
usa proceso real, sin red y con monitoreo pausado. Datos en volumen Linux temporal,
evidencia en target/, y eliminación del contenedor/volumen propios al terminar.
No abrir directamente la SQLite Linux con una conexión SQLite de Windows: el
ensayo copia la base detenida antes de inspeccionarla. Recuperación bajo Docker
no acredita configuración ni comportamiento de un servicio instalado en systemd.
