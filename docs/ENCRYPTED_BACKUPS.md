# Respaldos cifrados y recuperación

Estado: implementación candidata, no servicio productivo instalado. `backup.py`
genera un dump lógico PostgreSQL y un archivo de configuración, ambos cifrados
con age. La clave privada de recuperación no se instala en el servidor de origen.
El manifiesto completo solo aparece después de comprobar ambas operaciones y que
la configuración no cambió. Un directorio sin `manifest.json` es incompleto.

## Preparación de DevOps

1. En una estación de recuperación controlada, instalar age y ejecutar
   `age-keygen -o recovery.key`. Custodiar esa clave en una bóveda separada y una
   segunda copia recuperable. Exportar solamente el destinatario público con
   `age-keygen -y recovery.key > recipients.txt`. No poner claves en Git, tickets,
   logs, imágenes ni paquetes. Conservar las claves antiguas mientras existan
   respaldos cifrados para ellas.
2. En el host Linux, instalar Python 3.10 o posterior, age y Docker Compose.
   Usar un checkout de release propiedad de root en `/opt/monitor`, sin directorios
   padre modificables por otros usuarios. Los archivos de entrada deben pertenecer
   a root y no admitir escritura de grupo/otros. `.production` debe tener modo 0700.
3. Crear `/etc/monitor-backup` y `/var/backups/monitor` con propietario root y modo
   0700. Copiar **solo** `recipients.txt` a `/etc/monitor-backup/recipients.txt`,
   propietario root, modo 0600. No usar enlaces simbólicos. Mantener el destino
   fuera de `.production`. No rotar configuración/certificados durante el backup.
4. Ejecutar con los mismos nombre de proyecto, archivo de variables y overlays
   que el despliegue. Omitir las opciones opcionales que no se utilicen:

```bash
sudo python3 /opt/monitor/scripts/production/backup.py \
  --output-dir /var/backups/monitor \
  --recipients-file /etc/monitor-backup/recipients.txt \
  --project-name monitor-production-candidate
```

Las opciones `--env-file /ruta/variables.env` y `--compose-file /ruta/overlay.yml`
(repetible) conservan la configuración efectiva. No incluir valores secretos en
la línea de comandos. El dump usa el usuario de bootstrap dentro del contenedor
PostgreSQL, por su socket local; no abre un puerto de DB al exterior.

## Programación y copia fuera del servidor

DevOps debe crear un servicio systemd `Type=oneshot`, `User=root`,
`WorkingDirectory=/opt/monitor`, `UMask=0077` y `ExecStart` con el comando anterior.
Configurar `TimeoutStartSec=2h`, un timer con `OnCalendar=hourly` y
`Persistent=true`, y un manejador `OnFailure` conectado al sistema de alertas.
La frecuencia es un ejemplo: aprobar RPO y capacidad antes de habilitarla.

Se incluyen `scripts/production/monitor-backup.service` y
`scripts/production/monitor-backup.timer`. Revisar rutas, proyecto y opciones;
copiarlos como root a `/etc/systemd/system/` con modo 0644. Añadir receptor de
fallos mediante un drop-in, ejecutar `systemctl daemon-reload`, probar
`systemctl start monitor-backup.service` y revisar su resultado. Solo entonces
habilitar `systemctl enable --now monitor-backup.timer`. Comprobar programación
con `systemctl list-timers monitor-backup.timer`. La unidad local no sustituye
la transferencia externa ni su alerta de antigüedad.

Después del éxito, copiar el directorio completo a almacenamiento externo cifrado
en tránsito, con identidad restringida y protección contra borrado/inmutabilidad.
Copiar el manifiesto al final y comprobar tamaño/SHA-256 de los dos archivos en el
destino. Alertar si falla la copia o falta un respaldo externo reciente. El script
no configura proveedor, credenciales, transferencia ni alertas por sí mismo.
Un respaldo solo en el mismo disco **no satisface** la recuperación ante pérdida
del host. No borrar automáticamente respaldos hasta aprobar retención y demostrar
recuperación; restringir también el acceso al manifiesto (contiene metadatos).

## Verificación en el host de recuperación

Usar un directorio privado propiedad de root, archivos sin escritura para terceros
y la clave privada con modo 0600. Esta operación consume los datos descifrados sin
guardarlos en disco:

```bash
sudo python3 /opt/monitor/scripts/production/verify_backup.py \
  --backup-dir /srv/recovery/RESPALDO \
  --identity-file /srv/recovery/recovery.key
```

Exige manifiesto completo, tamaños/hashes, autenticación age hasta el final del
stream, cabecera de dump PostgreSQL y rutas de archivo seguras. El manifiesto no
está firmado: conservar su hash/identidad en un catálogo externo protegido para
detectar sustitución por otro respaldo válido. El resultado
`database_restore_verified: false` es deliberado: descifrar no demuestra restaurar.

## Ensayo de recuperación completo pendiente de aceptación

1. Elegir respaldo completo y verificarlo como arriba. Registrar hora, commit e
   ID de imagen PostgreSQL del manifiesto. Recuperar las imágenes por digest y
   usar las mismas versiones PostgreSQL/Timescale, sin actualizar durante el ensayo.
2. Preparar host/red/volúmenes vacíos y aislados de agentes y servicios reales.
   Descifrar la configuración a almacenamiento privado; inspeccionar antes de
   extraer, conservando permisos restrictivos. `config/` corresponde a
   `.production/`; `deployment/` conserva Compose y overlays como referencia.
   Las rutas relativas de Compose se interpretan desde el checkout del release,
   no desde el directorio del archivo de respaldo.
3. Recrear los roles desde `init-roles.sql` y preparar una DB limpia con Timescale.
   Aplicar el procedimiento oficial de restauración lógica de **esa versión**,
   incluyendo `timescaledb_pre_restore()` / `timescaledb_post_restore()` y sin
   restauración paralela (`-j`), según la [guía oficial](https://www.tigerdata.com/docs/deploy/self-hosted/backup-and-restore/logical-backup).
   Alimentar `pg_restore --exit-on-error` con la salida de `age -d`;
   exigir éxito de ambos procesos, sin ejecutar sobre la DB original.
4. Verificar esquema, hypertables/chunks, trabajos de retención, privilegios,
   identidades y deduplicación; comprobar MFA, sesiones revocadas, cuentas,
   dispositivos y métricas. Arrancar la aplicación solo al terminar la restauración.
5. Registrar tiempo real de recuperación, punto de datos recuperado y pérdida
   máxima observada. Reconciliar mensajes antes de volver a admitir agentes.

El respaldo lógico no incluye mensajes pendientes de RabbitMQ, outboxes de los
agentes, imágenes, certificados externos al árbol ni un archivo WAL/PITR. Definir
su recuperación y el RPO de extremo a extremo; no asumir que volver a la DB de
una hora anterior recupera mensajes que ya fueron confirmados por el broker.
La configuración y la DB no constituyen una instantánea transaccional conjunta.

## Evidencia disponible

`tests/encrypted_backup.py`: ensayo aislado Linux con age y PostgreSQL 16; cifrado,
restauración y comparación exacta de 5000 filas; también Timescale/PostgreSQL 15
con 5000 muestras y chunks comprimidos conservados. Rechazo de clave equivocada,
cifrado alterado incluso con checksum recalculado, rutas inseguras y productor
fallido. No certifica recuperación completa de identidades/aplicación, copia externa ni RPO/RTO productivos.
