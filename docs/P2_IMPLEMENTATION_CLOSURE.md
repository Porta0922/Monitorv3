# Cierre de implementación P2 y entrada en P4 — 2026-10-28

Estado: código implementado; validación funcional pendiente. No se han instalado
servicios, aplicado migraciones ni publicado cambios en esta entrega.

## Principales y sesiones concurrentes

El relay heredado conserva una concesión exclusiva. El registro opcional
AGENT_SESSION_REGISTRY permite credenciales y puertos loopback independientes por
principal (UID Linux / SID Windows). Solo el servicio lee los descriptores; el usuario
solo recibe su propia configuración. Registro, activación, revocación y rotación exigen
servicio y agentes detenidos. Un journal bloquea el arranque hasta completar escrituras.
Los registros se cargan al iniciar el servicio, no en caliente.

Windows, desde un paquete administrado y PowerShell elevado:

```powershell
.\multi-session.ps1 -Action enroll -SessionUser 'EQUIPO\usuario' -Port 47660
.\multi-session.ps1 -Action activate -SessionUser 'EQUIPO\usuario'
# Otros usuarios reciben otros puertos y claves.
.\multi-session.ps1 -Action rotate -SessionUser 'EQUIPO\usuario'
.\multi-session.ps1 -Action revoke -SessionUser 'EQUIPO\usuario'
.\multi-session.ps1 -Action repair
```

Linux, como root, con el servicio y sesiones detenidos:

```sh
python3 multi-session.py enroll --user usuario --port 47660
python3 multi-session.py activate --user usuario
python3 multi-session.py rotate --user usuario
python3 multi-session.py revoke --user usuario
python3 multi-session.py repair
```

Importar configuración de sesión existente requiere --import-legacy / -ImportLegacy.
Conserva literalmente una clave de caché explícita; si antes dependía del secreto IPC,
lo fija antes de rotar. Deshabilita el relay heredado global para no dejar una vía con
la credencial anterior. No cambiar el principal asociado a una cola ni copiar colas
entre máquinas: el cifrado también depende del UUID del equipo y del ID del SO.
Las credenciales identifican al principal enrolado; no certifican la identidad humana
que está usando el escritorio ni impiden que ese principal falsifique sus propios eventos.

El MSI instala un launcher HKLM Run: se ejecuta sin elevación en cada escritorio con
Explorer, incluido RDP, y sale inmediatamente si el SID no está activado. El DEB instala
un launcher XDG Autostart condicionado al UID. Ambos tienen reintento acotado ante fallo,
no inician sesiones ni habilitan linger. Escritorios sin Explorer/XDG Autostart necesitan
integración del administrador; no se promete autostart en shells que no ejecutan esos mecanismos.
Los launchers heredados se conservan para compatibilidad, pero el mutex de Windows y
el lock de sesión de Linux evitan duplicados dentro de un mismo escritorio.

Sesiones del mismo principal comparten su outbox persistente: SQLite WAL, espera de
bloqueo y event_id estable permiten entregas repetidas idempotentes. No se abandona la
cola al cerrar una sesión RDP; otra ejecución del mismo principal puede terminar la entrega.
Cada evento mantiene su boot_id original como identificador de ejecución, incluso en replay.
No se presentan esos identificadores como IDs WTS ni como sesiones humanas de login.

## Contrato y mantenimiento

Migración 0011: session_events separa actividad de principales nuevos de los totales
heredados de máquina. La atribución procede del listener autenticado, no del payload.
El servidor confirma la transacción completa junto con deduplicación. Los recursos de
CPU/RAM siguen siendo exclusivos del servicio. El histórico de sesiones conserva
actividad, aplicaciones y resúmenes de entrada; el panel consulta agregados por día UTC,
principal y ejecución, con paginación. Horas-persona no se suman como horas del equipo.
No hay reconstrucción de identidad para eventos heredados sin origen verificable.

SESSION_EVENT_RETENTION_DAYS=30 elimina detalle por lotes mediante el mantenimiento
histórico existente, solo al activar HISTORY_RETENTION_ENABLED. 0 desactiva ese trabajo.
No se generan archivos externos ni una segunda copia histórica indefinida. Tras la
retención, esos días no están disponibles en el panel; no se inventan resúmenes ausentes.
El estado de salud de sesiones permanece en la fila actual del equipo, sin historial.
Hasta 16 instancias recientes aparecen en diagnóstico; el servicio aplica límite de tamaño
con truncamiento explícito. El registro acepta hasta 64 principales, no presencia ilimitada.

## Diagnósticos y límites de captura

USB, copias USB, WiFi, inventario, osquery, recursos y ventanas informan resultados/errores.
Los fallos de comandos WiFi/USB ya no se convierten en listas vacías sanas. Linux enumera
ventanas reales; consulta la ventana activa por xprop sin elegir otra ventana visible.
La sesión informa errores de captura junto con su política y disponibilidad nativa.
Windows/Ubuntu/Debian conservan estados desconocido/no disponible cuando falta permiso,
escritorio o herramienta. osquery sigue siendo una dependencia opcional.

Los mapas de coordenadas NO están disponibles: los hooks actuales transportan categorías
agregadas, no coordenadas. Se retiró la publicación de una cuadrícula vacía con resolución
fija inventada. La UI recibe esta limitación; los contadores de entrada siguen disponibles.
Wayland sin acceso nativo también se declara no disponible. No son funciones certificadas
por las pruebas ni se intentará habilitarlas ocultamente durante P4.

## Migración y purga explícitas

packaging/common/data-lifecycle.py requiere Python 3.10 o superior, elevación/root,
servicios/agentes detenidos y rutas explícitas. Rechaza enlaces/reparse/hardlinks,
inventaría archivos reconocidos y comprueba SQLite/pending. No ejecutado en esta entrega.

```sh
python3 data-lifecycle.py plan --source /ruta/antigua --plan /ruta/admin/plan.json
python3 data-lifecycle.py migrate --source /ruta/antigua --plan /ruta/admin/plan.json --destination /ruta/nueva
```

La migración copia identidad (device_id.json), configuración/política y colas con sus
sidecars, verifica hashes, conserva principales/permisos y deja el origen intacto.
El destino debe ser nuevo. Después se configura AGENT_DATA_DIR con esa ruta mediante el
registro/reparación del servicio. No sobrescribe instalaciones existentes, no combina
identidades ni adivina claves faltantes. Un destino parcial se conserva para inspección;
no arrancar desde él. La decodificación de pendientes se valida en P4 antes del cambio definitivo.

manage-session migrate (Linux) / -Action migrate (Windows) reconcilia explícitamente las
claves de transporte de un enrolamiento heredado discordante desde la configuración de
sesión, conservando la clave original de outbox. Queda preparado y detenido. Si las claves
originales no están disponibles no existe recuperación automática legítima del cifrado.

Para purgar se genera un plan nuevo con agentes detenidos y se ejecuta la acción purge
con el mismo --source y --plan. Rechaza cambios y pendientes; --allow-pending-loss autoriza
explícitamente descartar pendientes. Solo elimina archivos reconocidos e inventariados,
no recorre directorios ni elimina perfiles, registros, binarios o archivos ajenos. Repetir
por cada directorio explícito de configuración/datos requerido; desconocidos se conservan.
Retirar autostart y servicio antes de purgar, para evitar recrear identidad/datos.
Para descriptores de principal o archivos adicionales se usa --include NOMBRE, repetido
idénticamente al crear el plan y al purgar. Solo acepta nombres, no subdirectorios.

## Paquetes y recuperación

DEB: prerm guarda flags de autostart y detiene los procesos del binario del paquete
usando pidfds; postinst restaura flags después de instalación/rollback. No inicia agentes.
Una operación interrumpida deja package-session-operation.json: package-sessions.py
restore restaura el autostart; remove finaliza retirada. Datos y claves se conservan.

MSI: install-package.ps1 verifica el paquete firmado y coordina -Action install/repair/remove.
Guarda un journal, deshabilita tareas/flags, detiene agentes, invoca Windows Installer y
restaura autostart tras éxito de instalación/reparación o rollback. En retirada, quita
las tareas administradas. Conserva una copia administrativa del coordinador fuera del
INSTALLFOLDER para recuperación si este desaparece. El servicio permanece detenido.
El MSI exige la propiedad del coordinador; es una barrera de flujo, no de seguridad frente
al administrador. Un corte deja el journal y bloquea el arranque hasta revisar el resultado
MSI y ejecutar package-sessions.ps1 restore/remove desde la copia de recuperación.
Esto combina transacción MSI y recuperación explícita de sesión; no es una transacción
atómica entre Windows Installer, tareas y archivos. El wrapper requiere el MSI firmado
correspondiente también para reparación/retirada.

La condición de lanzamiento se basa en [Launch de WiX](https://docs.firegiant.com/wix/schema/wxs/launch/).
Firma, paquetes reales, reinicio, rollback y permisos efectivos deben verificarse en P4.

## Orden de P4

1. Sintaxis y compilación final: Rust Windows, frontend; luego Ubuntu/Debian y binarios de pruebas.
2. Migraciones 0009–0011 y consultas de políticas/sesiones; actualización con legado y compresión.
3. Dos usuarios/sesiones concurrentes: identidad, claves, revocación, reinicios y recuperación offline.
4. SCM/systemd, MSI/DEB, firmas, actualización/rollback/cortes y migración/purga en datos desechables.
5. Retención/deduplicación/RabbitMQ, respaldo/restauración, accesibilidad, carga y piloto.

No ejecutar la purga ni instalar servicios en el equipo de trabajo como comprobación casual.
No publicar ni reemplazar GitHub hasta cerrar P4 y aprobar el despliegue.
