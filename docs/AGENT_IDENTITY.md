# Identidad, enrolamiento y renovación del agente

Actualizado: 2026-10-29. Código del bloque 2; despliegue productivo sujeto a la validación y autorización del TODO.

## Modelo de acceso

Cada credencial identifica un UUID de equipo y tiene vencimiento. El token aleatorio contiene 256 bits; PostgreSQL conserva únicamente su hash SHA-256. RabbitMQ mantiene su representación de contraseña y un usuario `agent_<credential_id>` sin etiquetas administrativas. Ese usuario no puede crear exchanges/colas ni consumir; solo puede publicar al exchange `monitoring` en `monitoring.<UUID>.<tipo permitido>`.

El publicador coloca su usuario en la propiedad AMQP `user_id`, que RabbitMQ contrasta con el usuario conectado. El consumidor vuelve a consultar la credencial y contrasta UUID del mensaje, UUID anidado y routing key antes de persistir. Las identidades inválidas se conservan en cuarentena/DLQ, no se incorporan a métricas. Los reintentos del servidor preservan la identidad original en una cabecera que solo se acepta cuando publica `monitor_server`. El agente no puede publicar como ese usuario ni escribir en las colas directamente.

El token no se incluye en eventos ni en la caché de telemetría. La API comprueba también el UUID solicitado para políticas, registro y actividad HTTP, y el propietario del comando para confirmaciones. El token de agente no permite navegar los datos del dashboard.

## Alta y recuperación

1. Instalar el servicio con los scripts de ciclo de vida. Conservar el UUID de `device_identity.json` y el directorio de datos; no regenerarlos al renovar o recuperar acceso.
2. Registrar ese UUID/hostname con un administrador mediante POST `/api/devices/register` si todavía no existe. El registro es una decisión administrativa, no prueba automática de propiedad de la máquina.
3. En el dashboard, abrir el equipo → Diagnóstico → Acceso del agente → Generar código de inscripción. Solo administradores ven este control. La inscripción se deshabilita cuando el servidor no tiene configurada la administración segura del broker; el piloto heredado no se convierte automáticamente. El código vence en una hora y habilita una sola credencial; generar otro invalida el anterior sin utilizar.
4. Entregar el código mediante un canal protegido y guardarlo en un archivo privado del equipo. Preparar la CA confiable que valida HTTPS y AMQPS. No distribuir credenciales del servidor ni del broker administrativo.
5. Ejecutar como administrador/root:

```sh
python3 scripts/configure-agent-identity.py --config /etc/activity-monitor/agent.env --data-dir /var/lib/activity-monitor --server-url https://monitor.ejemplo.com --code-file /ruta/protegida/enrollment.code --ca-file /ruta/protegida/ca.crt
```

En Windows usar `python` y rutas absolutas de la instalación. El helper restringe ACL a SYSTEM/Administradores; en Linux aplica 0600. Conserva la clave de caché existente; si hay caché sin clave explícita, rechaza el cambio para evitar volver ilegibles los pendientes. No reinicia ni reinstala el servicio: usar su script de ciclo de vida después de revisar la configuración. Las sesiones de usuario conservan sus scripts de enrolamiento y claves IPC independientes.

El agente crea y guarda un nuevo token antes de consumir el código, de manera cifrada y mediante reemplazo atómico. Si se pierde la respuesta puede repetir exactamente el mismo intento; reutilizar el código con otro token se rechaza. El código no permite leer ni reemplazar una credencial ya creada. El archivo `network-identity.json` queda cifrado con AES-GCM y una clave derivada de la clave privada de caché.

Para un equipo revocado o vencido, revocar sus credenciales activas anteriores y entregar un código nuevo en el mismo archivo. El agente reconoce el cambio y reintenta el enrolamiento cada minuto conservando UUID y outbox. Si se perdió la clave de caché, restaurarla desde custodia: cambiarla no descifra pendientes antiguos.

## Renovación y revocación

Las credenciales enroladas duran 30 días. El agente inicia renovación cuando quedan seis días; el servidor la permite en los últimos siete. Se admiten dos credenciales activas por equipo. La anterior conserva como máximo 48 horas de validez tras renovar, para recuperarse de una respuesta perdida. El intento queda persistido: repetirlo devuelve la misma credencial y no emite otras indefinidamente.

Una respuesta de renovación se guarda antes de cambiar el publicador. La reconexión usa la nueva identidad; las peticiones HTTP recargan la credencial. No se cambia UUID, clave de caché ni IDs de eventos, conservando la deduplicación. El desfase máximo de reloj futuro es 120 segundos y el horizonte de replay es 30 días, coherente con la retención de marcadores.

Revocar marca inmediatamente el registro en DB y elimina el usuario del broker, cerrando sus conexiones. Si RabbitMQ no responde, la API devuelve 202 indicando cierre pendiente: el consumidor ya rechaza esa identidad y un reconciliador reintenta cada 60 segundos. También elimina usuarios vencidos y huérfanos de operaciones incompletas; usa un bloqueo compartido con la provisión para no interferir con altas en curso. Las operaciones ya autorizadas antes de revocar pueden terminar; la revocación no deshace datos históricos.

Un equipo offline conserva sus pendientes cifrados. Si vuelve antes del vencimiento puede renovar y reenviar dentro del horizonte permitido. Si vuelve después, necesita un nuevo código. Los mensajes ya en el broker que se procesen con identidad vencida/revocada quedan en DLQ para investigación; no se importan automáticamente con una identidad nueva ni se borran silenciosamente.

## API administrativa

Ambos prefijos, con y sin `/api`, aplican la misma autorización:

| Método y ruta | Uso |
| --- | --- |
| POST /api/agent-credentials/enrollment/{device_id} | Código de un solo uso, válido una hora |
| GET /api/agent-credentials/{device_id} | Estado de hasta 100 credenciales recientes, sin hash/token |
| DELETE /api/agent-credentials/{device_id}/{credential_id} | Revocar; 204 confirmado, 202 cierre broker pendiente |
| POST /api/agent-credentials/{device_id} | Emisión manual avanzada, `valid_days` 1–90; preferir enrolamiento para el agente |
| POST /api/agent/enroll | Consumir código con device_id, code y new_token |
| POST /api/agent/renew | Renovar autenticándose con x-agent-token y enviando new_token |

La emisión, enrolamiento y revocación tienen auditoría sin secretos; las respuestas con material de acceso llevan no-store. Enrolamiento/renovación comparten un límite global de 120 intentos por minuto y proceso, con 429/Retry-After. Es un límite acotado, no un sistema distribuido anti-DDoS. HTTP limita cuerpos a 2 MiB; el broker/publicador limitan eventos a 1 MiB, dos conexiones y cuatro canales por usuario de agente. Las colas de telemetría y DLQ tienen un límite de 64 MiB por cola y rechazan publicaciones nuevas al llenarse, conservando las ya almacenadas. Los validadores específicos de telemetría mantienen sus límites más pequeños. La secuencia es diagnóstica y debe ser positiva; no se exige llegada estrictamente creciente porque se admite recuperación offline. La deduplicación conserva el ID estable de evento.

## TLS y custodia

`MONITOR_ENV=production` exige HTTPS en el agente y AMQPS en ambos extremos. Se valida certificado y hostname; no hay alternativa de confianza insegura. `SSL_CERT_FILE` permite una CA privada. El servidor necesita BROKER_ADMIN_URL (HTTPS privado), BROKER_ADMIN_PASSWORD_FILE y AGENT_BROKER_URL (endpoint AMQPS visible por el agente, sin credenciales). La cuenta `monitor_identity` solo se entrega al servidor; la interfaz de administración del broker no publica puerto externo.

Producción exige claves de caché/IPC explícitas de 32 bytes hex y bloquea el fallback de desarrollo. Respaldar claves y UUID por equipo en un almacén protegido y cifrado, separado de los datos. Rotar credenciales de red no implica rotar la clave de caché. El piloto en desarrollo conserva compatibilidad y no fue reinstalado.

Comprobación de vencimientos en Linux:

```sh
python3 scripts/production/tls-maintenance.py --warning-days 14
```

Integrar este comando en la supervisión del anfitrión; devuelve error si se aproxima el vencimiento. Tras obtener certificados renovados del emisor, el hook de despliegue es:

```sh
python3 scripts/production/tls-maintenance.py --install-from /ruta/protegida/certificados-renovados
```

Comprueba cadena, nombres, clave y vigencia; permite renovar hojas con la misma CA y recrea los servicios TLS del candidato para renovar los bind mounts. Guarda una copia privada durable durante la operación y restaura ante error. Una interrupción deja `.production/tls-renewal-pending` para recuperación administrativa; no se sobrescribe. Revisar y ejecutar `--rollback-pending` para restaurar el conjunto anterior si todavía es válido. El cambio de CA requiere distribución previa de confianza y no se automatiza con este comando. No se han emitido certificados reales ni configurado un emisor o tarea periódica en el anfitrión.

## Alcance de cierre

La implementación incluye HTTP, broker, agente, interfaz administrativa y herramientas de preparación/renovación. No equivale a autorización de producción: la aceptación en Windows/Linux instalados, el emisor real, firewall/DNS, custodia operativa y la campaña completa de seguridad pertenecen a los bloques de validación y operación. Ver la evidencia reciente en P4_VALIDATION.md.
