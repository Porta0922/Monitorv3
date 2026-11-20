# Recuperación aislada y transición de imágenes

Implementación 2026-11-16. Herramientas preparadas; no se ejecutó restauración real ni cambio de versión de la DB/broker del piloto. Las pruebas integrales quedan para la fase final por decisión del usuario.

## Restaurar DB y configuración sin tocar el despliegue activo

`scripts/production/restore_isolated.py` requiere Linux/root, Docker, age, el respaldo completo y la identidad privada de recuperación. Primero autentica ambos cifrados y compara hashes; después extrae manualmente solo archivos regulares bajo `config/` y `deployment/`, sin sobrescritura, enlaces ni rutas ascendentes. Impone 16 MiB por archivo y 64 MiB totales de configuración; cambiar esos límites exige revisión del caso concreto.

Usa exclusivamente el ID de imagen PostgreSQL registrado en el manifiesto, que debe estar disponible localmente. No descarga una versión alternativa ni realiza una actualización durante la recuperación. DevOps debe conservar esa imagen en su registro/archivo con el respaldo; el manifiesto no contiene los bytes de la imagen.

Crear un directorio vacío root/0700 en un volumen cifrado y ejecutar, completando las rutas reales:

```bash
install -d -m 0700 -o root -g root /srv/monitor-recovery/attempt-001
python3 /opt/monitor/scripts/production/restore_isolated.py \
  --backup-dir /var/backups/monitor/RESPALDO_COMPLETO \
  --identity-file /etc/monitor-recovery/identity.txt \
  --output-dir /srv/monitor-recovery/attempt-001
```

Todos los padres de esas rutas deben estar administrados por root y no ser escribibles por otros usuarios. La clave age debe ser privada. No usar una ruta con datos existentes como salida. El comando crea un contenedor `monitor-recovery-UUID` y su volumen propio, sin red, sin puertos y sin reinicio automático. La autenticación trust se usa únicamente dentro de ese contenedor aislado, accesible por Docker a administradores; no habilitar red ni publicar sus puertos.

Recrea roles desde el respaldo, restaura Timescale con `timescaledb_pre_restore()`/`timescaledb_post_restore()` cuando corresponde, ejecuta `pg_restore --exit-on-error` sin paralelismo y exige éxito del descifrado. Las sesiones humanas recuperadas se revocan para exigir un nuevo login; las identidades de agentes se conservan. Registra recuentos de dispositivos, credenciales, políticas, deduplicación, actividad, recursos y migraciones, e invariantes de referencias, roles y sesiones. No exporta los tokens ni las filas individuales.

Los respaldos nuevos incluyen `config/rabbit-live-definitions.json` cifrado y el ID de imagen RabbitMQ original. Si está presente, el recuperador exige esa imagen local, crea otro contenedor/volumen sin red, importa usuarios/permisos/vhosts/colas/bindings y elimina la cuenta temporal de bootstrap. Los respaldos anteriores sin esa entrada se reconocen como recuperación DB/configuración únicamente. Importar definiciones no recupera mensajes; el recibo mantiene `broker_messages_restored=false`.

Los contenedores se detienen al terminar, también ante errores. Los volúmenes y el directorio se conservan para revisión. `recovery.json` registra nombres exactos, imágenes, estado, recuentos y tiempo medido; no interpretarlo como aprobación productiva. Si hay `stop_failed`, aislar manualmente los recursos indicados antes de seguir. No se elimina automáticamente ningún volumen. La configuración extraída queda privada: antes de montarla en aplicaciones sin privilegios, preparar permisos apropiados en un destino separado siguiendo RELEASE_DEVOPS, sin volver públicamente legibles sus directorios.

## Lo que requiere aceptación posterior

- Comparar recuentos y datos del punto recuperado con las evidencias del respaldo; las consultas no prueban por sí solas equivalencia con la fuente.
- Revisar privilegios efectivos, MFA, login nuevo, políticas, identidad/enrolamiento y recepción completa en una aplicación aislada.
- Reconciliar las revocaciones de agentes posteriores al respaldo y rotar credenciales comprometidas. Un respaldo viejo no conoce esas revocaciones.
- Recuperar/reconciliar RabbitMQ y los outboxes de los agentes antes del corte: el dump no contiene sus mensajes. No declarar RPO cero ni volver a admitir productores sin reconciliación.
- Medir RPO/RTO integral, además del tiempo DB/configuración del recibo. Hostnames, certificados, rutas y configuración requieren adaptación controlada al destino.
- No arrancar directamente los Compose archivados sin revisar mounts, redes, puertos y versiones. No conectar los recursos de recuperación al piloto automáticamente.

El procedimiento automatiza DB, configuración e invariantes; la restauración integral de la aplicación y la continuidad de mensajes se aceptan en la campaña final. Referencia: [restauración lógica de Timescale](https://www.tigerdata.com/docs/deploy/self-hosted/backup-and-restore/logical-backup).

## Nuevas imágenes: instalación separada

`docker-compose.next.yml` construye el candidato endurecido **TimescaleDB 2.32.0 / PostgreSQL 17** desde `DockerPostgres/Dockerfile`, compartido por DB y mantenimiento. La base y RabbitMQ **4.2** están fijados por digest; el candidato final requiere registrar su propio digest antes de distribuir. Se aplica después de Compose base y producción, con volúmenes distintos (`postgres17_data`, `rabbitmq42_data`) y puertos loopback distintos (28443 HTTPS, 35671 AMQPS). El escaneo corregido y la migración sintética pasaron; no constituye aprobación productiva.

En un checkout de despliegue independiente, con `.production` y el env-file aprovisionados para ese destino, preparar un proyecto nuevo:

```bash
python3 scripts/production/start_fresh_candidate.py \
  --project monitor-next-office01 --env-file /etc/monitor-release/production.env
```

Sin `--start` solo revisa la configuración. Agregar `--start` construye y arranca después de rechazar contenedores/volúmenes existentes y mounts de datos inesperados. No ejecutarlo ahora como prueba si la campaña está diferida. El helper es para primer arranque, no mantenimiento de un proyecto existente; conservar todos los overlays en operaciones posteriores.

Para un piloto con datos PG15/Rabbit3, **no** basta con aplicar el overlay. RabbitMQ requiere broker nuevo, drenaje/transferencia, reconciliación y reversión según RABBITMQ_MIGRATION. PostgreSQL requiere plan de migración mayor separado. La matriz oficial indica que Timescale 2.28 es la última serie que admite PG15, mientras 2.32 admite PG17; preparar una versión puente compatible antes de migrar y actualizar la extensión después según el procedimiento oficial. No reutilizar el volumen PG15 con PG17 ni restaurar automáticamente el dump antiguo en una extensión diferente.

Antes de aprobar: revisar ruta exacta desde la versión instalada, obtener imágenes puente por digest, escanear artefactos/SBOM/procedencia, ensayar compatibilidad de mantenimiento/chunks/rollback y completar aceptación. Se conserva el Compose activo anterior; no se modificó el piloto. Referencias: [matriz Timescale/PostgreSQL](https://www.tigerdata.com/docs/deploy/self-hosted/upgrades/upgrade-pg), [migración RabbitMQ entre entornos](https://www.rabbitmq.com/docs/blue-green-upgrade).
# Ensayo del 2026-11-17

La recuperación aislada de DB, configuración y definiciones RabbitMQ pasó con
datos sintéticos: 5.000 filas comprimidas idénticas por SHA-256, credenciales y
políticas conservadas, sesiones web revocadas y recursos recuperados detenidos
sin red. Se corrigió una carrera de arranque: pg_isready debe comprobar TCP local
para no confundir el servidor temporal de inicialización con el servidor final.
Los errores de pg_restore quedan en `restore.private.log`, dentro del directorio
privado de recuperación, sin copiar sus contenidos a logs compartidos.

También pasó la migración sintética PG15/Timescale 2.28.3 → PG17/2.28.3 →
PG17/2.32.0, conservando los datos comprimidos. Esta evidencia no acredita
mensajes en tránsito, aplicación completa, RPO/RTO productivo ni aceptación de
un respaldo real de producción.
