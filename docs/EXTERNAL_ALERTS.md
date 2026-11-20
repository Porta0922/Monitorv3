# Alertas externas de operación

Implementación 2026-11-16; pendiente configuración del receptor y prueba de entrega al finalizar la preparación. No activada en el piloto ni conectada a un servicio externo.

## Alcance implementado

`scripts/production/notify_failure.py` envía un evento JSON mínimo cuando una unidad systemd falla. El respaldo incluye `OnFailure=monitor-alert@%n.service`. Los scripts de copia externa ya devuelven fallo al no poder verificar la publicación; ese fallo llega al mismo canal cuando se ejecutan desde la unidad de backup.

El cuerpo contiene versión del esquema, UUID de evento, tipo `monitor.unit.failed`, nombre de unidad, hostname y fecha UTC. No contiene comandos, logs, títulos de ventanas, datos de usuarios ni credenciales. Las tres tentativas conservan el UUID y cabecera Idempotency-Key para deduplicación del receptor. Respuestas 429/5xx y fallos de red admiten reintentos acotados. Un 2xx confirma aceptación HTTP, no que una persona haya recibido o atendido el aviso.

TLS se valida con raíces del sistema o una CA especificada; no admite desactivar validación, redirecciones ni proxies heredados. El token opcional se lee de un archivo privado y los errores no imprimen URL, token ni respuesta del receptor. El servicio tiene límite de 45 segundos y de arranques; no se configura una cadena recursiva de alertas.

## Preparación por DevOps

El receptor debe aceptar este JSON y transformar/enrutar los eventos hacia la herramienta organizacional. No asumir que el formato es directamente compatible con un webhook de Slack, Teams u otro proveedor. Aprovisionar URL/token mediante el gestor de secretos, fuera del repositorio.

En el anfitrión Linux, con privilegios administrativos:

```bash
install -d -m 0700 -o root -g root /etc/monitor-alert
# Crear webhook.json con el gestor de secretos y el formato indicado abajo.
chown root:root /etc/monitor-alert/webhook.json
chmod 0600 /etc/monitor-alert/webhook.json
install -m 0644 /opt/monitor/scripts/production/monitor-alert@.service /etc/systemd/system/
systemctl daemon-reload
```

Formato del archivo privado (ejemplo sin credenciales reales):

```json
{
  "url": "https://alerts.example.invalid/monitor/events",
  "bearer_token": "REEMPLAZAR_EN_EL_GESTOR_DE_SECRETOS"
}
```

`bearer_token` es opcional. `ca_file`, si hace falta, debe ser una ruta absoluta a un certificado raíz confiable administrado por root. La configuración y todos sus directorios padre deben pertenecer a root y no admitir escritura de otros usuarios; el archivo no admite lectura de grupo/otros ni enlaces simbólicos. No colocar el archivo bajo `/root`: la unidad usa ProtectHome.

Instalar el receptor antes de habilitar el timer de backup. Para otras unidades de Monitor, añadir en un drop-in:

```ini
[Unit]
OnFailure=monitor-alert@%n.service
```

Usar únicamente unidades con nombres simples. Si la alerta falla, la unidad notificadora queda fallida y el journal registra un mensaje genérico: supervisar ese estado con un monitor externo independiente. No hay cola persistente de notificaciones ni garantía de entrega mientras el host/receptor estén caídos. `OnFailure` no detecta servicios vivos pero degradados ni errores que el proceso no comunique con un estado fallido.

## Detector periódico implementado

`watch_health.py` y `monitor-watchdog.service/.timer` añaden lectura periódica del estado Docker del proyecto, salud del servidor, consumidores y acumulación por cola estándar, DLQ, tamaño de DB/disco, antigüedad/fallos de mantenimiento, respaldo reciente y señales agregadas de login fallido, limitaciones, accesos denegados, CSRF y eventos high/critical. Opcionalmente comprueba readiness HTTPS y vencimiento de certificados con `--https-url`, `--ca-file` y `--certificate` repetible. Sin esos parámetros no cubre la ruta TLS externa ni vencimientos; DevOps debe incorporarlos.

Las consultas DB son de solo lectura y con timeout; se ejecutan dentro del contenedor con el rol de bootstrap local desde el anfitrión administrador. No se añade una credencial de monitorización al navegador ni se abre un endpoint nuevo. El acceso Docker ya equivale a administración del anfitrión: ejecutar únicamente desde el host de operación protegido.

Se notifican códigos de alarma sin nombres de usuarios/equipos de oficina ni payloads de eventos. Cada proyecto necesita su directorio de estado propio. Hay exclusión mutua, estado persistente, avisos por cambio, recuperación con lista vacía y recordatorio cada 30 minutos por defecto; una entrega fallida no se marca como enviada. `--report-only` evita toda entrega y no modifica el estado de notificación. El detector devuelve 0 saludable, 1 alarmas y 2 entrega fallida; errores de configuración también fallan. El timer vuelve a ejecutarlo sin OnFailure duplicado.

Instalar unidad/timer después de ajustar `ExecStart` al proyecto real, vhost, ruta de datos Docker y backups. Añadir URL HTTPS/CA y certificados del destino; ajustar umbrales al tamaño de la oficina. Valores iniciales: 20 eventos de auth/CSRF/acceso en 15 minutos, cualquier limitación de cuenta/evento high/critical/DLQ, 10.000 mensajes por cola, mantenimiento/backup con más de dos horas, DB 100 GiB, disco libre menor de 5 GiB o 10%, certificados a menos de 14 días. No son límites universales. Una cola sin consumidor alarma incluso vacía; las colas no estándar requieren ampliar la configuración/código explícitamente.

```bash
install -m 0644 /opt/monitor/scripts/production/monitor-watchdog.service /etc/systemd/system/
install -m 0644 /opt/monitor/scripts/production/monitor-watchdog.timer /etc/systemd/system/
# Ajustar ExecStart mediante un drop-in antes de habilitar.
systemctl daemon-reload
# En el despliegue aceptado: systemctl enable --now monitor-watchdog.timer
```

El backup reciente se reconoce por manifiesto local completo; no se descifra en cada minuto ni eso acredita recepción de copia externa. El fallo de publicación externa lo comunica la unidad de backup. El detector no es un IDS ni cubre ataques que la aplicación no haya registrado. Los eventos se evalúan en ventana móvil: no constituye un registro de entrega individual de cada evento. Supervisar desde fuera la ausencia de ejecuciones del timer/host, porque un host caído no puede avisar de su propia caída.

## Aceptación final

### Correo mediante SMTP con TLS

El notificador admite también `transport: "smtp"` en el mismo archivo privado
`/etc/monitor-alert/webhook.json`; no requiere cambiar las unidades systemd.
Ejemplo sin secretos reales:

```json
{
  "transport": "smtp",
  "smtp_host": "smtp.example.invalid",
  "smtp_port": 465,
  "smtp_mode": "tls",
  "smtp_username": "monitor@example.invalid",
  "smtp_password": "APROVISIONAR_CON_GESTOR_DE_SECRETOS",
  "from": "monitor@example.invalid",
  "to": ["operaciones@example.invalid"]
}
```

Para puerto 587 usar `smtp_mode: "starttls"`: el cifrado es obligatorio antes
de autenticarse. No se admite SMTP sin TLS. Se valida el certificado y puede
configurarse `ca_file` con las mismas restricciones que el webhook. Un relay
autorizado que no requiera autenticación puede omitir ambos campos de credenciales.
La aceptación SMTP no demuestra entrega en la bandeja del destinatario. Las
tentativas reutilizan Message-ID, pero SMTP no garantiza deduplicación.

DevOps debe aprovisionar una cuenta de servicio o contraseña de aplicación
admitida por su proveedor. Este transporte no implementa OAuth. Nunca copiar
la contraseña personal a ejemplos, Compose, Git ni al navegador. Los fallos
registran categorías fijas sin respuestas del servidor ni credenciales.

Estado del piloto al 2026-11-17: TLS/EHLO funciona, pero la conexión se cierra al
autenticar. El usuario confirmó que proporcionó su contraseña habitual, no una
contraseña de aplicación. No se harán nuevos intentos con esa credencial.
No se habilitó el envío periódico ni se confirmó entrega.

Para Gmail/Workspace, DevOps debe habilitar un relay autorizado o usar una
contraseña de aplicación si la política de la organización lo permite. Para la
prueba local: activar verificación en dos pasos, generar una contraseña de
aplicación para Monitor y reemplazar únicamente el valor privado
`CONTRASEÑA_EMAIL` en `.local-test/.env`. No enviarlo por chat ni guardarlo en Git.
Si la opción no aparece, solicitar al administrador el relay autorizado; no
desactivar la seguridad de la cuenta. Después realizar un único envío de prueba
y confirmar recepción antes de habilitar el timer. En producción, aprovisionar
el JSON privado descrito arriba mediante el gestor de secretos.

Referencias: [contraseñas de aplicación de Google](https://support.google.com/mail/answer/185833?hl=es)
y [SMTP para aplicaciones en Workspace](https://knowledge.workspace.google.com/admin/gmail/send-email-from-a-printer-scanner-or-app).

- En la fase final, iniciar una notificación sintética con un nombre de unidad de ensayo y confirmar recepción/deduplicación con el responsable; probar rechazo de CA, redirecciones, credenciales inválidas y receptor caído.
- Validar detectores, cooldown, recuperación, persistencia y señales de autenticación con fixtures al terminar la implementación; no se ejecutaron todavía.
- Registrar receptor, responsables, escalamiento y tiempos de respuesta. No cerrar el bloque productivo con el canal escrito pero sin recepción demostrada.
