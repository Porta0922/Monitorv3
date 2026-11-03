# Despliegue candidato de producción

Actualizado: 2026-10-29. Preparación implementada; producción no aprobada. El piloto existente no fue modificado.

## Tres controles preparados

1. **PostgreSQL:** `monitor_bootstrap` se reserva para inicializar una base nueva. `monitor_owner` ejecuta el trabajo separado `migrate`; `monitor_app` tiene DML y lectura del registro de migraciones; `monitor_maintenance` solo accede a las tablas necesarias para retención y resúmenes. Aplicación y mantenimiento no tienen propiedad de tablas, CREATE de esquema, superusuario ni administración de roles. El servidor comprueba versiones y checksums antes de aceptar tráfico, sin ejecutar DDL en producción. El trabajo de migraciones reaplica `scripts/production/grants.sql` después de migrar; debe finalizar correctamente para arrancar el servidor.
2. **Procesos sin root:** servidor, dashboard, migraciones y mantenimiento usan UID/GID 65532, filesystem de solo lectura, `/tmp` temporal, capacidades retiradas y `no-new-privileges`. Tienen límites de procesos/memoria y logs acotados. PostgreSQL y RabbitMQ conservan el arranque de sus imágenes oficiales; no se afirma que todo el stack arranque sin root. Los targets Docker predeterminados mantienen compatibilidad con el piloto.
3. **Transporte:** Nginx escucha exclusivamente HTTPS en 8443 con TLS 1.2/1.3 y HSTS; RabbitMQ deshabilita AMQP en claro y escucha AMQPS en 5671. El servidor exige URL `amqps` en producción y recibe la CA mediante `SSL_CERT_FILE`. No se desactiva validación de certificados. La identidad individual usa credenciales por equipo, permisos de topics y verificación del publicador; no usa mTLS. El proxy hacia la API y PostgreSQL todavía usan transporte interno sin TLS.

## Puertos del candidato

| Servicio | Publicación en anfitrión |
| --- | --- |
| Dashboard HTTPS | 127.0.0.1:18443 → 8443 |
| RabbitMQ AMQPS | 127.0.0.1:25671 → 5671 |
| API, PostgreSQL, administración RabbitMQ | Ninguna |
| Redis | Excluido por defecto, perfil `unused-redis` |

Los dos puertos publicados solo permiten acceso local o por un túnel autorizado. No se habilita todavía la conexión de equipos remotos. No cambiar el bind a todas las interfaces para saltarse los criterios pendientes.

## Preparación en Linux

Requisitos: Python 3, OpenSSL, Docker Compose con `!reset`/`!override`, cuentas administradas con hashes Argon2id y certificados emitidos para los nombres reales. El JSON de cuentas debe incluir un administrador activo; ver [control de acceso](ACCESS_CONTROL.md).

Preparar fuera del repositorio `ca.crt`, `web.crt`, `web.key`, `broker.crt`, `broker.key`. Los `.crt` de servidor deben incluir la cadena intermedia necesaria. La CA debe validar ambas cadenas. El certificado del broker necesita además el nombre DNS interno **rabbitmq** en SAN. No usar certificados de ensayo en producción. Las claves deben permitir arranque desatendido y guardarse bajo permisos restringidos.

```sh
python3 scripts/prepare-production.py --users-file /ruta/protegida/users.json --tls-dir /ruta/protegida/tls --web-host monitor.ejemplo.com --broker-host broker.ejemplo.com
docker compose --project-name monitor-production-candidate -f docker-compose.yml -f docker-compose.production.yml config --quiet
```

El generador comprueba cadena, nombre, correspondencia de clave pública y certificado, y vigencia superior a un día. Rechaza Windows, claves incompatibles y `.production` existente. No imprime secretos, no inicia servicios ni instala confianza en clientes. Un error puede dejar archivos parciales: revisar antes de reintentar. La herramienta `scripts/production/tls-maintenance.py` comprueba vencimiento y renueva hojas con rollback, sin cambiar la CA. Integrarla con el emisor y la supervisión del anfitrión forma parte de la puesta en producción.

`.production` y sus directorios privados tienen modo 0700. Los archivos montados individualmente son 0444 para que los procesos sin root puedan leerlos; el directorio de cuentas montado es 0555. La restricción de acceso en el anfitrión depende del padre 0700. Los secretos se entregan únicamente a los servicios que los necesitan. Compose no los cifra: administradores del anfitrión conservan acceso. Proteger disco y backups.

**Solo base y volúmenes nuevos:** los scripts de inicialización no se ejecutan sobre un volumen existente. No reutilizar los del piloto ni regenerar sus claves. La conversión de una instalación anterior requiere backup restaurable y un procedimiento específico de roles/propiedad/credenciales.

El arranque futuro en un anfitrión aislado con los requisitos preparados es:

```sh
docker compose --project-name monitor-production-candidate -f docker-compose.yml -f docker-compose.production.yml up -d --build
```

No se ejecutó este despliegue durante este incremento. Mantener intacto `monitorv32-local`. Antes de cada actualización, ejecutar de nuevo el trabajo de migraciones con la imagen nueva, comprobar su éxito y después actualizar el servidor; no iniciar la aplicación con un esquema anterior. Preparar backup y ventana de mantenimiento antes de migrar datos reales.

## Identidades y administración privada del broker

El generador prepara una cuenta `monitor_identity` con privilegios de administración del broker y secreto exclusivo del servidor, separado de `monitor_server`, que consume/reintenta telemetría. La API administrativa usa HTTPS interno en 15671, sin puerto publicado. El servidor crea usuarios por credencial, con dos conexiones/cuatro canales y publicación únicamente al topic del equipo. Las colas de telemetría/DLQ tienen un límite de 64 MiB por cola con rechazo de publicaciones nuevas; no descartan mensajes antiguos para hacer espacio.

`AGENT_BROKER_URL` se escribe en `.production/endpoint.env` usando `--broker-host` y `--broker-port` (25671 por defecto). Debe resolver al endpoint accesible por el agente; los binds de loopback del candidato continúan intactos. Preparar DNS/firewall/ingreso antes de conectar máquinas remotas. El servidor requiere `BROKER_ADMIN_URL`, `BROKER_ADMIN_PASSWORD_FILE` y CA válida; nunca entregar este secreto a agentes.

## Validación estricta de arranque

`MONITOR_ENV` admite development/test/production. Producción exige archivos JWT, URL DB, URL RabbitMQ y cuentas administradas; rechaza token HTTP compartido (AGENT_AUTH_TOKEN/AGENT_AUTH_TOKEN_FILE), valor directo y archivo simultáneos, secretos vacíos/de ejemplo, contraseña administrativa de bootstrap, cuenta DB distinta de `monitor_app` y RabbitMQ sin AMQPS. Los secretos son una línea UTF-8 de hasta 4096 bytes. El modo `--migrate` requiere una URL propia con usuario `monitor_owner`.

CORS productivo no autoriza orígenes cruzados; el dashboard usa el mismo origen mediante Nginx. Las URLs completas de DB no se imprimen al arrancar. Nginx añade HSTS, nosniff, protección de marcos y política de referencia. Incluye CSP y sesiones protegidas con CSRF; administradores requieren MFA TOTP. `MONITOR_PUBLIC_ORIGIN` debe ser el origen HTTPS exacto del dashboard, incluido el puerto no estándar. Ver ACCESS_CONTROL.md para alta y recuperación de cuentas.

## Pendientes antes de producción

- Certificados reales, emisor/renovación operativa, confianza de clientes y aceptación HTTPS/AMQPS en los equipos objetivo. El recorrido integrado local con certificados sintéticos y publicador Rust ya pasó.
- Distribución y aceptación operativa del enrolamiento/renovación implementados; ver [identidad](AGENT_IDENTITY.md). Validar dimensionamiento para la cantidad real de equipos.
- Arranque completo en Linux con estos permisos, imágenes y recursos; separación de redes y endurecimiento de servicios de infraestructura.
- Migración de instalaciones existentes, procedimiento operativo de actualización/rollback, backups cifrados y restauración comprobada.
- Revisión de dependencias/imágenes, privacidad, observabilidad y aceptación del [TODO vigente](TODO.md).

La versión fuente del agente sigue en 0.1.0: estos cambios son exclusivos de servidor y despliegue; el agente instalado del piloto no se altera.
