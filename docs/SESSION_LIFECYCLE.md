> Actualización 2026-10-28: el cierre de implementación, los nuevos flujos multiusuario
> y la integración de paquetes están descritos en [P2_IMPLEMENTATION_CLOSURE.md](P2_IMPLEMENTATION_CLOSURE.md).
> Las limitaciones y pendientes de esta página anteriores a esa fecha son históricos;
> la validación funcional continúa en P4.

# Ciclo de vida de la sesión de escritorio

Implementación en código para un usuario enrolado por equipo. No habilita soporte multiusuario/RDP ni sustituye los paquetes MSI/DEB pendientes. Los comandos de esta página no se ejecutaron durante la implementación.

## Diseño de claves y recuperación

El enrolamiento nuevo genera una clave de transporte IPC y otra distinta para la cola local. En enrolamientos anteriores sin AGENT_OFFLINE_CACHE_KEY, rotate fija primero la clave antigua como clave de caché y luego cambia la de transporte. Preserva literalmente su representación porque la derivación de caché utiliza el texto, UUID y machine ID. No cambia la identidad ni recifra/elimina eventos pendientes. Claves personalizadas fuera del formato hexadecimal de 64 caracteres requieren una migración explícita.

Los scripts mantienen session-operation.json bajo el directorio administrativo, accesible solo al administrador. Contiene las configuraciones necesarias para completar la operación, incluidos secretos: no subirlo a Git ni adjuntarlo a diagnósticos. Cada archivo se reemplaza atómicamente; no existe una transacción de filesystem que abarque ambos archivos. repair completa la operación guardada si hubo interrupción. El journal se retira después del marcador final.

El agente actualizado rechaza el arranque mientras ese journal exista junto a su configuración o hasta tres directorios superiores. Los binarios antiguos no tienen esta protección: actualizar antes de utilizar estas operaciones. Los scripts requieren detener el servicio y la sesión; no deben iniciarse manualmente mientras operan. Se excluyen otras operaciones administrativas mediante flock/mutex.

Acciones:
- repair: termina el journal pendiente o reescribe la configuración concordante preservando claves y estado. No adivina ante claves divergentes sin journal.
- activate: habilita el relay de una sesión preparada; deja procesos detenidos.
- rotate: cambia el secreto IPC de servicio/sesión conservando la clave de cola y el estado activo/preparado.
- revoke: deshabilita el relay y cambia solo la clave del servicio. Conserva configuración/clave/cola del usuario para recuperación.
- reenroll: autoriza nuevamente al mismo UID/SID revocado con una clave IPC nueva; conserva la clave de caché y queda preparado hasta activate. No cambia de usuario ni elimina sus datos.

## Linux (Ubuntu/Debian)

Usar los scripts desde un directorio administrado por root. Instalación inicial del usuario:

```sh
sudo python3 packaging/linux/enroll-session.py --user USUARIO
sudo python3 packaging/linux/manage-session.py activate
sudo python3 packaging/linux/session-autostart.py enable
```

El servicio debe estar detenido para enrolar/activar/rotar/reparar. Para cambios posteriores, detener también la unidad de usuario desde la sesión correspondiente:

```sh
systemctl --user stop activity-monitor-session.service
sudo systemctl stop activity-monitor-agent.service
sudo python3 packaging/linux/manage-session.py rotate
# Si una operación se interrumpe:
sudo python3 packaging/linux/manage-session.py repair
```

Autostart instala una unidad de usuario global, condicionada por un marcador administrativo exclusivo del UID enrolado. Se ejecuta al activar graphical-session.target, sin habilitar linger ni iniciar colectores en usuarios no enrolados. Requiere escritorio con ese target de systemd; no garantiza autostart en escritorios que no lo integran. La unidad reinicia ante fallo con límite de intentos. El script no inicia la sesión inmediatamente.

Retirada:

```sh
sudo python3 packaging/linux/session-autostart.py disable
sudo systemctl stop activity-monitor-agent.service
sudo python3 packaging/linux/manage-session.py revoke
```

Disable elimina únicamente el marcador y detiene nuestra unidad si existe el gestor de usuario. No detiene agentes lanzados manualmente. La plantilla global permanece disponible, pero no se activa sin marcador. La configuración y la outbox permanecen intactas.

## Windows (PowerShell elevado)

Mantener todo el directorio packaging/windows en una ubicación administrada: la tarea utiliza start-session.ps1 de esa ubicación, que debe permanecer disponible.

```powershell
.\packaging\windows\enroll-session.ps1 -SessionUser 'EQUIPO\usuario'
.\packaging\windows\manage-session.ps1 -Action activate
.\packaging\windows\session-autostart.ps1 -Action enable -BinaryPath 'C:\Monitor\activity-monitor-agent.exe'
```

La tarea se ejecuta como el usuario interactivo, sin elevación, al iniciar sesión. No almacena contraseña, no usa SYSTEM y no inicia una sesión de escritorio. Un solo proceso por tarea; reinicia ante fallo con un intervalo de un minuto. start-session.ps1 propaga el código de salida del agente. Esto no proporciona aislamiento por sesión RDP concurrente.

Para rotar/reparar, detener primero la tarea y el servicio. manage-session rechaza agentes que sigan ejecutándose desde la ruta registrada.

```powershell
Stop-Service ActivityMonitor
.\packaging\windows\manage-session.ps1 -Action rotate
# Ante operación interrumpida:
.\packaging\windows\manage-session.ps1 -Action repair
```

Retirar autostart y revocar:

```powershell
.\packaging\windows\session-autostart.ps1 -Action disable -BinaryPath 'C:\Monitor\activity-monitor-agent.exe'
Stop-Service ActivityMonitor
.\packaging\windows\manage-session.ps1 -Action revoke
```

Disable solo retira una tarea cuyo nombre, principal, ejecutable y argumentos coincidan con el enrolamiento. Si cambió la ruta del paquete, requiere resolver esa diferencia explícitamente. No elimina caché, identidad o credenciales, ni mata procesos por nombre. Si queda un agente manual, debe detenerse antes de revocar.

## Límites pendientes

- Empaquetado MSI/DEB e integración de estos comandos en sus flujos.
- Firmas, recuperación de actualización binaria interrumpida y migración de instalaciones ajenas a estas rutas.
- Multiusuario/RDP y atribución por usuario/sesión. Diagnóstico de instancias consolidado implementado en SESSION_HEALTH.md; no equivale a multiusuario.
- Purga explícita de datos: no se implementa eliminación automática de colas pendientes.
- Validación funcional de permisos, reinicio, fallos intermedios, conservación de pendientes y gestores reales en P4.
