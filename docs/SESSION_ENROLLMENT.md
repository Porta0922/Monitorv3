# Enrolamiento inicial del componente de sesión

Implementado en código; no ejecutado ni validado. No es todavía un MSI/DEB ni un
mecanismo de actualización/revocación. Mantener la activación para P4.

## Condiciones comunes

El servicio debe estar instalado y detenido. Enrolar un único usuario de escritorio;
no ejecutar el modo interactive heredado junto con service+session. Se usa la configuración
estándar del servicio; despliegues con rutas/identidades diferentes necesitan migración.

Los scripts generan 32 bytes aleatorios cuando no hay clave, o conservan la clave
existente si es válida y única. Rechazan claves ambiguas/incorrectas, otro enrolamiento
y configuraciones de sesión existentes. Nunca imprimen claves ni copian credenciales
AMQP/API al perfil de sesión. Conservan datos, identidad y cachés del agente.

Ambos dejan AGENT_SESSION_RELAY_ENABLED=false. Preparan archivos; no arrancan procesos,
no registran autostart y no cambian firewall. El marcador session-enrollment.json guarda
principal, estado y ruta, sin secretos. Un estado preparing tras interrupción requiere
reparación explícita: no se continúa ni rota claves automáticamente.

## Linux Ubuntu/Debian

Script: packaging/linux/enroll-session.py (Python 3 estándar).
Recibe --user y opcionalmente --port. Requiere root, cuenta/grupo activity-monitor y
/etc/activity-monitor/.env de root, sin escritura de grupo ni acceso de otros usuarios.

- Configuración del servicio: root:activity-monitor, 0640.
- Directorios de enrolamiento: controlados por root, 0711 para permitir recorrido
  sin conceder lectura de las credenciales del servicio.
- Sesión: /etc/activity-monitor/sessions/<uid>/session.env, propiedad del usuario, 0600.
- Marcador administrativo: root, 0600.

No escribe como root dentro del HOME del usuario ni agrega al usuario al grupo del
servicio. Rechaza enlaces simbólicos en destinos y escribe mediante temporal+fsync+
replace en directorios controlados por root. La unidad user-systemd usa %U para
seleccionar la configuración del UID que la ejecuta.

## Windows

Script: packaging/windows/enroll-session.ps1, PowerShell elevado.
Recibe -SessionUser (cuenta resoluble a SID) y opcionalmente -Port. Exige ActivityMonitor
detenido y configuración en ProgramData/ActivityMonitor/.env.

Rechaza rutas reparse y configuraciones/carpetas con propietario o permisos de escritura
no administrativos. No modifica recursivamente ACL de datos/logs existentes.

- Servicio y marcador: SYSTEM/Administrators con control total.
- SessionEnrollment: administración completa y recorrido para el usuario inscrito.
- SessionEnrollment/<SID>/session.env: SYSTEM/Administrators con control total;
  usuario inscrito solo lectura, sin herencia de permisos generales.
- Temporales con secretos se escriben en el directorio administrativo protegido.

El launcher start-session.ps1 recibe la ruta resultante y corre bajo el usuario inscrito.
No lo ejecutar como SYSTEM. Todavía falta registrar y supervisar el inicio de sesión
desde el paquete definitivo. Las ACL y las rutas deben validarse con usuarios reales en P4.

## Activación, revocación y recuperación pendientes

Tras P4 se habilitará el relay y se arrancarán ambos roles. Una revocación completa
necesita detener la sesión, deshabilitar su inicio, retirar acceso y gestionar la clave
sin perder la outbox. Rotarla directamente no basta: también cifra la cola de sesión
si no se configuró una clave de caché independiente. Esa migración sigue pendiente.

No borrar el marcador ni reemplazar la clave para inscribir otro usuario sin ese
procedimiento. Multisesión/RDP no está resuelto por estos scripts.

## Validaciones reservadas a P4

Permisos efectivos con usuario autorizado/no autorizado, no exposición de secretos,
rutas enlazadas/reparse, servicio activo, enrolamiento repetido, interrupción entre pasos,
conservación de clave/identidad/outbox y operación posterior bajo cuentas restringidas.
