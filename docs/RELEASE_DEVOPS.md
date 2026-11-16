# Guía de release para DevOps

Actualizada: 2026-11-07. Las credenciales reales se incorporarán en el servidor de destino por DevOps, según decisión del usuario. Esta guía prepara el procedimiento; no declara aprobado el release. Consultar los bloqueantes vigentes en [TODO.md](TODO.md).

## 1. Definir y registrar el destino

Usar un anfitrión Linux dedicado con Docker Engine, Compose que admita `!reset` y `!override`, Python 3, OpenSSL, Git, reloj sincronizado y disco cifrado. Dar acceso al grupo Docker solo a administradores: permite controlar el anfitrión.

Registrar en el ticket de release:

| Dato | Ejemplo o ubicación |
| --- | --- |
| Commit aprobado y CI | SHA completo, enlace al PR y a sus ejecuciones |
| Dashboard | `monitor.empresa.example`, puerto público 443 |
| Broker | `broker.empresa.example`, puerto AMQPS definido por DevOps |
| Red permitida | VPN/subredes de equipos y administración |
| Custodia | Gestor de secretos y responsables de recuperación |
| Operación | Receptor de alertas, ventana de cambio, RPO/RTO |
| Datos | Retención aprobada, exclusiones de captura, aviso a usuarios |
| Paquetes | Versión de agente, sistemas admitidos, firmas y SHA256 |

No promover un commit con CI fallido ni sustituir un paquete entregado por otro de igual versión. La fuente del agente es `agent/Cargo.toml`; el piloto instalado sigue en 0.1.0. [Reglas de versionado](VERSIONING.md).

## 2. Obtener y construir el candidato

Trabajar desde un checkout dedicado. Los ejemplos siguientes son Bash en Linux; ejecutar con una cuenta administrativa y completar los valores del ticket, sin poner contraseñas en comandos.

```bash
set -euo pipefail
git clone https://github.com/Porta0922/Monitorv3.git /opt/monitor
cd /opt/monitor
read -r -p 'SHA completo aprobado: ' RELEASE_SHA
git checkout --detach "$RELEASE_SHA"
test "$(git rev-parse HEAD)" = "$RELEASE_SHA"
python3 scripts/check-publication.py
docker build --target production -f DockerServer/Dockerfile \
  -t "monitor-server:$RELEASE_SHA" .
```

Antes de promover, generar SBOM y escanear las imágenes completas mediante [IMAGE_SECURITY.md](IMAGE_SECURITY.md). Registrar sus IDs/digests en el ticket. La etiqueta local por SHA ayuda a identificar el build, pero no garantiza reproducibilidad: conservar también las imágenes aprobadas en un registro privado por digest. No asumir que reconstruir después producirá bytes idénticos.

## 3. Preparar cuentas sin escribir contraseñas en archivos de texto

Crear un directorio privado **fuera del checkout**. En estos ejemplos se utiliza `/root/monitor-release`; ejecutar esta sección como root. No habilitar `set -x`, grabar la terminal ni publicar capturas con códigos MFA.

```bash
install -d -m 0700 /root/monitor-release /root/monitor-release/tls
python3 scripts/manage-accounts.py \
  --users-file /root/monitor-release/users.json \
  --docker-image "monitor-server:$RELEASE_SHA" \
  create administrador --role admin \
  --provisioning-file /root/monitor-release/administrador-mfa.txt
```

La herramienta solicita la contraseña de forma interactiva, guarda Argon2id y genera el material de alta MFA en el archivo privado indicado. Entregarlo al administrador mediante el canal seguro de la organización y registrarlo en su autenticador. Nunca incorporarlo al README, GitHub, ticket ni navegador como una API key. Custodiar la recuperación y retirar la copia de provisión según la política de secretos.

Crear cuentas personales `viewer` u `operator` mediante el mismo comando y su rol correspondiente. Preparar un administrador de emergencia con `--emergency` y MFA independiente: queda deshabilitado hasta una recuperación autorizada. Ver [ACCESS_CONTROL.md](ACCESS_CONTROL.md).

## 4. Obtener certificados reales

Colocar en `/root/monitor-release/tls`:

| Archivo | Contenido |
| --- | --- |
| `ca.crt` | CA/bundle que valida web y broker; nunca su clave privada |
| `web.crt` | Certificado web con cadena intermedia y SAN del dashboard |
| `web.key` | Clave privada web para arranque desatendido |
| `broker.crt` | Certificado con SAN del broker público **y `rabbitmq` interno** |
| `broker.key` | Clave privada del broker |

Para el nombre interno `rabbitmq`, usar una PKI que pueda emitirlo y distribuir su confianza a los clientes. Una CA pública normalmente no emite nombres internos. No eliminar la validación del nombre para resolver ese problema. El generador exige actualmente un bundle que valide ambas cadenas.

## 5. Generar la configuración privada

**Solo para instalación con volúmenes nuevos.** No ejecutar sobre el piloto ni regenerar claves de una instalación existente. El generador rechaza `.production` existente; conservarlo e investigar en lugar de borrarlo para repetir.

```bash
python3 scripts/prepare-production.py \
  --users-file /root/monitor-release/users.json \
  --tls-dir /root/monitor-release/tls \
  --web-host monitor.empresa.example --web-port 443 \
  --broker-host broker.empresa.example --broker-port 5671
docker compose -p monitor-production-candidate \
  -f docker-compose.yml -f docker-compose.production.yml config --quiet
```

Los nombres y puertos son los **visibles por usuarios/agentes**, no necesariamente los puertos internos del contenedor. El comando valida TLS, crea claves aleatorias y escribe:

| Ubicación dentro de `.production/` | Uso / quién lo recibe |
| --- | --- |
| `secrets/db_password` | Bootstrap PostgreSQL; no se entrega a la aplicación |
| `secrets/database_url` | Conexión de `monitor_app`, usada por server |
| `secrets/migration_url` | Conexión de `monitor_owner`, usada por migrate |
| `secrets/maintenance_password` | Cuenta limitada de mantenimiento |
| `secrets/rabbitmq_url` | Consumidor/reintentos `monitor_server` |
| `secrets/broker_admin_password` | Administración de identidades del broker, solo server |
| `secrets/jwt_secret` | Secreto del servidor; nunca frontend o agente |
| `accounts/users.json` | Cuentas, hashes, MFA y época de sesión |
| `endpoint.env` | `AGENT_BROKER_URL` y `MONITOR_PUBLIC_ORIGIN`, sin credenciales |
| `init-roles.sql` | Bootstrap de roles; contiene contraseñas, tratar como secreto |
| `rabbit-definitions.json` | Definiciones y hashes del broker; tratar como secreto |
| `tls/` | Certificados y claves montados en servicios específicos |
| `deployment.json` | Nombres usados por mantenimiento TLS |

La carpeta padre es privada; algunos archivos montados son legibles por el UID del contenedor. No hacer `chmod -R 777`. No usar el `.env` de desarrollo ni agregar `ADMIN_PASSWORD`, `AGENT_AUTH_TOKEN` o secretos `VITE_*` a producción. Compose monta secretos desde disco; no los cifra. Integrar su provisión y respaldo con el gestor de secretos de la organización.

Cambiar una contraseña en el archivo **no rota automáticamente** la cuenta ya creada en PostgreSQL o RabbitMQ. Coordinar cambio del servicio, secreto y recreación de consumidores; probarlo y documentar la reversión antes de rotar.

## 6. Red, retención y configuración de operación

El overlay publica solo `127.0.0.1:18443` para HTTPS y `127.0.0.1:25671` para AMQPS. Mantener DB, API y administración RabbitMQ sin puertos externos. Redis fue retirado de la configuración activa: la aplicación no lo utiliza.

DevOps debe configurar el ingreso de red/VPN hacia esos endpoints, conservando TLS extremo a extremo. Si se publican interfaces específicas mediante un overlay privado, limitar origen con firewall del anfitrión y de Docker; no confiar únicamente en reglas que Docker pueda saltarse. Comprobar desde una máquina autorizada y otra no autorizada. El proxy HTTP adicional debe respetar el origen público y el modelo de IP usado por límites de peticiones; ajustar para oficinas detrás de NAT. No exponer el backend como ruta alternativa al dashboard.

Definir retención antes de habilitar borrado histórico. Configurar variables de mantenimiento en un archivo privado pasado con `--env-file` a **todos** los comandos Compose del despliegue. Variables disponibles: `METRICS_RAW_RETENTION_DAYS`, `METRICS_SUMMARY_RETENTION_DAYS`, `HISTORY_RETENTION_ENABLED`, `ACTIVITY_RETENTION_DAYS`, `SESSION_EVENT_RETENTION_DAYS`, `USB_RETENTION_DAYS`, `WIFI_RETENTION_DAYS`, `AUDIT_RETENTION_DAYS`, `SECURITY_EVENT_RETENTION_DAYS`, `INVENTORY_CHANGE_RETENTION_DAYS`, `RESOLVED_ALERT_RETENTION_DAYS`, `DEDUPE_RETENTION_DAYS`. No reducir deduplicación por debajo del horizonte de replay. [Política y límites](DATA_RETENTION.md).

## 7. Arranque y comprobaciones

Usar el mismo nombre de proyecto en toda la operación. El helper TLS acepta `--project-name`, `--env-file` y `--compose-file` repetible; pasar exactamente la configuración del despliegue para no perder overlays de red o retención al renovar. Su proyecto predeterminado es `monitor-production-candidate`.

```bash
docker compose -p monitor-production-candidate \
  -f docker-compose.yml -f docker-compose.production.yml build
docker compose -p monitor-production-candidate \
  -f docker-compose.yml -f docker-compose.production.yml up -d
docker compose -p monitor-production-candidate \
  -f docker-compose.yml -f docker-compose.production.yml ps -a
```

Comprobar que `migrate` finalizó con código 0; server no debe arrancar con migraciones fallidas. Inspeccionar logs de manera privada. Validar readiness, acceso HTTPS con CA y nombre correctos, MFA, roles y sesión; sin `curl -k` ni bypass de certificados. Verificar cookies Secure/HttpOnly/SameSite, cierre de sesión, CSRF, límites de login, archivos sensibles denegados y ausencia de secretos en assets/almacenamiento del navegador.

## 8. Enrolar equipos

Distribuir únicamente instaladores genéricos firmados y aprobados. No redistribuir el MSI privado del piloto. Crear el código temporal del equipo desde Administración del dashboard y guardarlo en un archivo privado del equipo. Nunca entregar la URL administrativa del broker ni secretos del servidor.

Ejemplo Linux, adaptando rutas a la instalación real:

```bash
sudo python3 scripts/configure-agent-identity.py \
  --config /etc/activity-monitor/agent.env \
  --data-dir /var/lib/activity-monitor \
  --server-url https://monitor.empresa.example \
  --code-file /ruta/protegida/enrollment.code \
  --ca-file /ruta/protegida/ca.crt
```

En Windows usar Python y rutas absolutas de la instalación desde una terminal administrativa. El script conserva UUID, clave de caché y pendientes; no reinicia. Reiniciar mediante el procedimiento de ciclo de vida del instalador y validar servicio **y sesión de usuario**: el servicio aislado no ve todas las ventanas del escritorio. Comprobar recursos, ventanas, actividad, desconexión/reconexión y ausencia de equipos duplicados. Ver [AGENT_IDENTITY.md](AGENT_IDENTITY.md) y [PLATFORM_LIFECYCLE.md](PLATFORM_LIFECYCLE.md).

## 9. Operación, actualización y reversión

- Integrar vencimiento TLS: `python3 scripts/production/tls-maintenance.py --warning-days 14`. Alertar al responsable ante salida no cero. Renovar con `--install-from /ruta/protegida/renovados` y los mismos `--project-name`, `--env-file` y `--compose-file` del despliegue; la rotación de CA requiere distribución previa de confianza.
- Alertar externamente por API/consumidor caído, cola/DLQ creciente, disco/DB, fallos de mantenimiento/backup y abuso de autenticación. Registrar una alerta realmente recibida; disponer de la pantalla no equivale a vigilancia operativa.
- Configurar los [respaldos cifrados](ENCRYPTED_BACKUPS.md), con clave de recuperación separada, programación y copia externa supervisada. Restaurar Timescale en un entorno limpio y medir RPO/RTO antes de aprobar el release. Copiar un volumen activo sin coordinación no acredita un backup consistente.
- Antes de actualizar: conservar imágenes por digest, configuración y backup restaurable. Construir/obtener el nuevo candidato; ejecutar `docker compose ... run --rm migrate` con **la imagen nueva**, verificar código 0 y luego recrear server/dashboard con esa misma versión. Conservar todos los overlays, `--env-file` y nombre de proyecto.
- Revertir solo si el esquema es compatible con la versión anterior. Si no lo es, seguir restauración ensayada y reconciliar mensajes; cambiar únicamente la etiqueta del servidor no revierte la DB.
- No cambiar RabbitMQ 3.12 por 4.2 sobre el mismo volumen. Seguir [RABBITMQ_MIGRATION.md](RABBITMQ_MIGRATION.md). No ejecutar `down -v` en producción como procedimiento de actualización o recuperación.

## 10. Acta de release

Adjuntar commit, CI, versiones/digests, firmas/hashes de instaladores, matriz Windows/Linux, resultados de seguridad y carga, restauración, recepción de alertas y responsable de rollback. Confirmar revocación del token GitHub histórico; eliminarlo del árbol actual no lo invalida. Registrar riesgos pendientes con responsable y fecha; no declarar listo mientras queden bloqueantes del TODO.

Las credenciales se completan en destino. Los scripts y la documentación pueden prepararse antes; DNS, firewall, certificados reales, custodia, firma y aceptación operativa deben verificarse allí.
