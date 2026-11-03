# Cuentas, sesiones y recuperación

Actualizado: 2026-11-03. El piloto limpio incorpora el bloque 3 con bootstrap de desarrollo local. Su cuenta admin no tiene MFA: el campo de código se deja vacío. El perfil productivo exige cuentas individuales y MFA administrativo como se describe abajo.

## Acceso y permisos

Producción exige `AUTH_USERS_FILE`, cuentas individuales Argon2id y MFA TOTP para todos los administradores habilitados. Rechaza bootstrap `ADMIN_PASSWORD` y JWT humanos enviados como Bearer. Desarrollo conserva compatibilidad con bootstrap explícito y Bearer para el piloto. No utilizar ese modo como producción.

| Rol | Alcance |
|---|---|
| viewer | Consultar equipos y sus datos; gestionar sus propias sesiones |
| operator | Lo anterior, exportar CSV, abrir/liberar investigaciones y resolver alertas |
| admin | Lo anterior, administrar equipos, políticas, comandos, credenciales, DLQ, cuentas y auditoría |

Son roles de una sola organización, con lectura de todos sus equipos. No existe aislamiento por departamento o tenant. Otorgar lectura implica acceso a los datos de actividad recopilados; las reglas de minimización corresponden al bloque 5.

## Sesiones del navegador

El login devuelve identidad, rol, vencimiento y token CSRF; nunca devuelve la credencial de sesión. El navegador recibe una cookie opaca de 256 bits, cuyo hash se almacena en PostgreSQL. En producción se llama `__Host-monitor_session`, con `Secure`, `HttpOnly`, `SameSite=Strict`, `Path=/` y sin Domain. En desarrollo se usa `monitor_session` sin Secure para HTTP local.

Vencimiento absoluto: ocho horas. Inactividad: treinta minutos sin peticiones autenticadas; el polling del dashboard cuenta como actividad. Cada solicitud comprueba cuenta, generación, vencimiento, revocación y rol. Cambiar contraseña o MFA invalida las sesiones; la herramienta administrativa incrementa `session_epoch` en cada cambio, por lo que deshabilitar y rehabilitar no resucita sesiones anteriores. Una reducción de rol observada se conserva en la sesión aunque se vuelva a promover la cuenta.

El dashboard guarda el CSRF solo en memoria y restaura la sesión mediante `GET /api/auth/session`. Borra el antiguo `auth_token` de localStorage. Las mutaciones exigen `X-CSRF-Token`; el login exige `X-Monitor-Client: dashboard`. Se rechazan orígenes cruzados; `MONITOR_PUBLIC_ORIGIN` debe coincidir exactamente con el origen HTTPS público, incluido el puerto no estándar, sin barra final. No se autorizan peticiones CORS de otros orígenes.

La pantalla **Sesiones y acceso** permite cerrar una sesión o todas las propias. El administrador puede revocar las de otra cuenta, revisar su estado MFA y consultar intentos fallidos. La API no expone hashes ni semillas MFA.

## Límites y auditoría

Login: cinco intentos por cuenta normalizada en quince minutos, compartidos mediante DB; un login correcto reinicia ese contador. Además, 120 intentos por minuto por proceso y dos cálculos de contraseña simultáneos. API humana: 600 solicitudes por usuario/minuto/proceso. La respuesta de credenciales inválidas es genérica. El sexto intento genera evento y aviso; la pantalla administrativa muestra fallos de quince minutos y bloqueos de veinticuatro horas. No se implementó envío de correo/SIEM: esa integración operativa pertenece al bloque 10.

Auditoría central: actor, acción, destino, resultado y fecha. Las operaciones sensibles registran intención antes de ejecutarse y resultado después; se incluye el UUID válido del equipo cuando aparece en ruta, consulta o cuerpo. No se copian cuerpos, query strings, contraseñas, cookies ni semillas. Si falla el registro inicial, no se ejecuta la operación; si falla el registro final, se registra error operativo y `x-monitor-audit-incomplete: true`. Lectura de auditoría exclusiva de administradores. Es un registro operativo en DB, no un almacén inmutable frente al administrador de PostgreSQL.

Retención propuesta de auditoría: `AUDIT_RETENTION_DAYS=180`, mediante el trabajo de mantenimiento existente, con aprobación y restauración todavía pendientes del bloque 5. Limpieza de autenticación cada diez minutos: sesiones vencidas hace más de un día, contadores de login de más de un día y pasos MFA de más de noventa días. El log local de la herramienta de cuentas queda protegido con 0600; incorporarlo a la política de rotación de logs del anfitrión, máximo 180 días, al desplegar.

Nginx productivo añade CSP, HSTS, nosniff y protección de marcos. Los scripts se cargan desde el mismo origen; el inicializador del tema es un archivo externo. Se permite estilo inline porque los gráficos lo utilizan. CSV entrecomilla campos, escapa comillas y neutraliza prefijos de fórmula.

## Herramienta administrativa Linux

`scripts/manage-accounts.py` se ejecuta como root en el anfitrión productivo. Usa únicamente la biblioteca estándar de Python y el binario del servidor para Argon2id mediante stdin. No coloca contraseñas en argumentos ni imprime semillas. Requiere un ancestro privado root 0700; `.production` cumple esa función. El directorio `accounts` se monta completo, de solo lectura, de modo que el servidor ve reemplazos atómicos de `users.json` sin reiniciarse.

Preparación inicial, antes del generador de despliegue:

```sh
sudo install -d -m 0700 /root/monitor-access
sudo python3 scripts/manage-accounts.py --users-file /root/monitor-access/users.json --server-bin /ruta/activity-monitor-server create administrador --role admin --provisioning-file /root/monitor-access/admin-mfa.txt
```

Importar el URI del archivo MFA en el autenticador mediante un canal privado; no usar generadores QR externos ni adjuntarlo a tickets. Custodiar la recuperación en el almacén de secretos autorizado y retirar la copia de incorporación cuando deje de ser necesaria. Después pasar `users.json` a `prepare-production.py` junto con los certificados. Cada persona necesita su propia cuenta.

Sobre el despliegue preparado, usar `--users-file .production/accounts/users.json` y `--docker-container NOMBRE_DEL_SERVER` en lugar de `--server-bin`. El contenedor debe contener el servidor actualizado con `--hash-password-stdin`. Acciones:

- `create nombre --role viewer|operator|admin`: solicita contraseña; administradores requieren `--provisioning-file` nuevo.
- `disable nombre` / `enable nombre`: cambia disponibilidad y generación de sesiones.
- `role nombre --role ...`: cambia permisos e invalida sesiones; al promover sin MFA exige archivo de incorporación.
- `reset-password nombre`: contraseña nueva e invalidación de sesiones.
- `reset-mfa nombre --provisioning-file /root/monitor-access/nueva-mfa.txt`: nueva semilla e invalidación de sesiones.
- `create emergencia --emergency --provisioning-file /root/monitor-access/emergencia-mfa.txt`: administrador deshabilitado, con contraseña y MFA independientes.

Mantener la cuenta de emergencia deshabilitada y sus factores bajo custodia separada. Ante pérdida de MFA: identificar al titular por el procedimiento organizativo, usar acceso administrativo al anfitrión para restablecer MFA, incorporar el factor nuevo y comprobar login. Si se habilitó la cuenta de emergencia, deshabilitarla después y revisar auditoría. No hay bypass MFA ni códigos universales. No deshabilitar el último administrador operativo sin preparar recuperación; el arranque productivo exige al menos uno habilitado.

La herramienta bloquea modificaciones concurrentes, escribe atómicamente, hace fsync e incrementa la generación. Su log adyacente registra intención y aplicación sin secretos. Si se interrumpe, revisar el archivo vigente y ese log antes de repetir; puede quedar un archivo de incorporación aún no aplicado.

## Evidencia y límites

Ver `P4_VALIDATION.md`. El recorrido aislado usa cuentas/datos sintéticos y no cambia el piloto. La aceptación en navegadores, el procedimiento con los responsables reales, certificados reales y despliegue completo quedan para los bloques 9 y 10.

Referencias del diseño: [OWASP sesiones](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html), [OWASP CSRF](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html), [RFC 6238](https://www.rfc-editor.org/rfc/rfc6238).
