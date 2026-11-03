# Servicio y componente de sesión

Implementado en código, no compilado ni ejecutado. Todas las pruebas están en P4.
El canal permanece desactivado por defecto y no se instaló ningún componente.

## Propiedad y responsabilidades

| Función | service | session | interactive heredado |
| --- | --- | --- | --- |
| Crear identidad del equipo | Sí | No, la solicita al servicio | Sí |
| CPU/RAM, inventario, USB/WiFi, seguridad | Sí | No | Sí |
| Ventanas, aplicaciones abiertas, entrada | No | Según capacidad del escritorio | Según capacidad |
| RabbitMQ y credenciales remotas | Sí | No | Sí |
| Comandos, políticas remotas, actualizador y UI web | Sí | No | Sí |
| Cola local | agent_service_cache.db | session_outbox.db por usuario | agent_user_cache.db |

El modo interactive es una alternativa heredada de un solo proceso. No debe convivir
con service+session. Migrar autostarts antiguos antes del despliegue; esta entrega no
los desinstala automáticamente. La separación es de procesos/roles en el mismo binario;
se conserva un solo artefacto para empaquetar posteriormente.

## IPC local

Se usa TCP **exclusivamente en 127.0.0.1**, puerto configurable (47652 predeterminado),
para compartir el transporte en Windows y Linux sin nuevas dependencias. No es una
API HTTP. Cada petición y respuesta se cifra/autentica con AES-256-GCM del crate ya
existente, nonce aleatorio de 96 bits, protocolo y dirección dentro del mensaje.
La respuesta debe coincidir con el UUID de petición. No se transmite la clave.

Cada máquina necesita una clave aleatoria de 32 bytes, representada por 64 caracteres
hexadecimales. El servicio y un usuario de sesión autorizado reciben esa misma clave
en archivos distintos. No se usa un secreto global ni un valor predeterminado. La
protección frente a otros usuarios locales depende de mantener esos archivos privados.
No protege frente a un administrador local que pueda leer ambos procesos.

El servidor limita cada frame a 256 KiB, a 8 conexiones y a 5 segundos por operación.
Solo acepta activity, running_apps, input_heatmap e input_summary en rutas explícitas.
Valida identidad, metadatos, fecha futura y contadores; elimina campos de recursos de
input_summary y añade su propia identidad/credencial remota. La sesión no puede enviar
comandos ni inventario a través del canal.

Primera ejecución: requiere servicio accesible para recibir el UUID. La sesión guarda
session_identity.json y no crea una identidad alternativa. Arranques posteriores
pueden recoger offline con esa identidad. Un cambio del UUID exige migración explícita;
el servicio rechaza eventos dirigidos a otro equipo. Una identidad corrupta detiene
el arranque. La escritura usa sustitución atómica; los permisos Windows automáticos siguen pendientes.

## Entrega

1. El productor guarda un sobre con event_id estable en su SQLite cifrado.
2. La sesión reintenta entregar lotes de hasta 256 eventos cada 2 segundos.
3. El servicio inserta en su propia caché con synchronous=FULL antes de responder.
4. La sesión marca sincronizado solo el event_id confirmado.
5. El servicio publica a RabbitMQ con confirmación y conserva pendientes ante fallo.

Aceptar localmente **no confirma commit en PostgreSQL**. La atomicidad de deduplicación
más escritura en el servidor sigue en P1/P4. No se promete entrega exactamente una vez.
La deduplicación local usa event_id y conserva filas sincronizadas durante tres días;
tras limpieza puede existir reenvío del mismo ID, que deberá resolver el servidor.

La cola ya no borra pendientes antiguos al alcanzar 20.000 eventos ni reemplaza la DB
ante cualquier error. Devuelve error y conserva lo existente. Un productor sin espacio
puede perder muestras nuevas; el fallo se registra. No equivale a capacidad ilimitada.
No se cambió el formato AES-GCM de caché ni se migraron eventos existentes.

## Una sesión productora

Una concesión en memoria de 90 segundos permite a una sola instancia entregar eventos
a la vez; otra instancia mantiene su cola pendiente. La concesión se reinicia junto
con el listener. Esta es una protección básica, **no agregación multiusuario**:
configurar un único usuario de escritorio por máquina. RDP simultáneo, cambio rápido
de usuario y reproducción de intervalos superpuestos necesitan contrato posterior.

## Configuración y arranque preparados

- Servicio: AGENT_MODE=service, AGENT_SESSION_RELAY_ENABLED=true y clave/puerto locales.
- Sesión: AGENT_MODE=session y AGENT_CONFIG_FILE absoluto de configuración exclusiva.
  Debe fijarse el modo antes de arrancar; no copiar el .env del servicio.
- Linux: archivo del usuario 0600 obligatorio. Plantilla user-systemd ligada a
  graphical-session.target; confirmar DISPLAY/XAUTHORITY importados en P4. Los datos
  usan XDG_DATA_HOME/activity-monitor-session o ~/.local/share/activity-monitor-session.
- Windows: datos en LOCALAPPDATA/ActivityMonitorSession; launcher preparado, sin registro
  automático ni tarea de inicio todavía. La ACL del archivo debe admitir únicamente
  usuario inscrito, SYSTEM y administradores; empaquetado/provisionamiento pendientes.

Las rutas personalizadas deben ser absolutas. El supervisor cancela tareas hijas al
cerrar y no las reinicia después de shutdown. En Unix se mantiene flock por carpeta
para impedir dos procesos usando el mismo estado; Windows conserva el mutex de sesión.

## Límites pendientes

La sesión sincroniza y persiste la política del servicio (AGENT_POLICIES.md). Faltan
revocación, validación del enrolamiento/permisos iniciales y persistencia de concesiones.
El enrolamiento inicial preparado se describe en SESSION_ENROLLMENT.md.
La salud de sesión no está consolidada aún en salud del equipo. Hooks, UI y otros
workers heredados necesitan completar cierre coordinado. Cambiar la clave con cola
pendiente requiere migración de cifrado, no editar el archivo sin más.

No habilitar antes de P4: compilación Windows/Linux, frames inválidos/replay/tamaño,
permiso de archivos, disco lleno, ACK perdido, reinicios, caída del servicio/broker,
identidad cambiada, doble sesión, cierre y conservación de CPU/RAM sin duplicados.
