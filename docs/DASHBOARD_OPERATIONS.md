# Operación del dashboard y preparación de versiones

Implementación: 2026-10-28. Pendiente de validación P4. No se ha aplicado a producción.

## Incorporación y organización

Administración explica la incorporación del servicio desde un paquete firmado aprobado
por TI, configuración protegida y primera señal. No es un generador de credenciales ni
un enrolador remoto. Los paquetes firmados aún deben producirse y validarse en P4.
Desde Equipos → Administrar se asignan nombre, un grupo (área/oficina) y versión objetivo.
Los grupos se derivan de equipos existentes: un grupo sin equipos desaparece del filtro.
No hay políticas por grupo. Las columnas se recuerdan solo en el navegador actual.
Búsqueda literal, filtros y orden se conservan en URL; el servidor devuelve 20 equipos
por página y cuenta el conjunto completo. Las otras pantallas conservan sus contratos.

## Políticas y versiones

Administración permite consultar la política global; Administrar equipo abre la excepción
individual. La excepción reemplaza la política completa, no mezcla campos. Quitarla
restaura herencia global; quitar la global restaura defaults del servidor.
Solo admin puede escribir, con motivo obligatorio y auditoría en la misma transacción.
Una revisión concurrente se rechaza: actualizar, revisar y volver a guardar.
El bloqueo de escritura tiene límite de cinco segundos.
Guardar significa persistido. El servicio consulta periódicamente (300 s); la sesión
recibe su configuración mediante IPC. Se confirma coincidencia del contenido con la
huella reportada por el servicio y diagnóstico reciente (900 s), no por haber pulsado
Guardar. Esto no acredita captura ni aplicación en todas las sesiones de usuario.
La versión objetivo es seguimiento, no una orden de actualización. Comparación exacta
con la versión reportada; no se interpreta orden semántico ni se instala remotamente.
Las cuentas se administran mediante AUTH_USERS_FILE; ver API_AUTHORIZATION.md.
La consola incluye consulta administrativa de usuarios, roles y desactivaciones sin hashes.
Las altas y modificaciones siguen siendo operaciones del archivo protegido montado en
solo lectura; no se ha implementado edición web de credenciales.

## Interpretación de métricas

Conectado, señal atrasada y desconectado son estados diferentes. Supervisado indica
vida de tarea; captura completada/error/sin observación es una señal separada con fecha.
USB, WiFi, inventario y aplicaciones reportan su última observación cuando su tarea se
 ejecuta. Una lista vacía en un adaptador sin error explícito no acredita disponibilidad.
Los demás adaptadores conservan desconocido, además del diagnóstico específico de
entrada y estados por campo de recursos. Completar adaptadores y consolidación de
sesiones sigue en P2. Una observación puede preceder una pausa o reinicio del recolector.

La migración 0010 añade disponibilidad nullable al detalle de entrada. NULL significa
legado/desconocido. Los resúmenes conservan muestras disponibles/no disponibles y
calculan las desconocidas por diferencia. Son cantidades de muestras, no horas de
cobertura; cero actividad no demuestra disponibilidad. No se reconstruye el pasado.

## Despliegue posterior y recuperación

En P4: respaldo verificado, migraciones 0009/0010 con servidor y mantenimiento de la
misma revisión, dashboard y luego agentes. No ejecutar un mantenimiento antiguo contra
las nuevas columnas: no preservaría los conteos de disponibilidad al compactar.
Ensayar primero sobre Timescale, incluidas tablas comprimidas. Una reversión de binario
requiere revisar compatibilidad de esquema; no borrar columnas ni revertir datos a mano.
RabbitMQ y outbox conservan su política de entrega. Retenciones siguen opt-in.

## Preparación reproducible de release

Usar una revisión Git limpia, Cargo --locked, npm ci, rust-toolchain.toml y las imágenes
fijadas por digest. Registrar versiones exactas de compilador, SO/SDK, WiX, OpenSSL,
Node/npm y herramientas de paquetes en un JSON de entorno por destino. No incluir secretos.
Usar SOURCE_DATE_EPOCH del commit para herramientas que lo admitan. Construir en entornos
fijados; generar y verificar firmas antes de calcular los hashes de distribución.

scripts/release-manifest.py recibe --artifact (repetible), --target, --environment y
--output. Produce JSON ordenado con commit, época, hashes de locks/configuración y
SHA256/tamaño de cada artefacto. Rechaza fuentes sucias, nombres duplicados y salida
existente. Guardar artefactos, entorno y manifiesto fuera del checkout. Este script no
construye, firma ni publica. El manifiesto también requiere firma al distribuirlo.
Dos construcciones independientes y comparación de resultados pertenecen a P4; un
manifiesto determinista no garantiza binarios idénticos, especialmente con firmas selladas.

## Pendiente de P4

Migraciones y compactación con legado NULL; filtros/paginación concurrentes; roles y
conflictos de políticas; huellas después de pausa/reinicio/offline; datos atrasados;
accesibilidad y dispositivos móviles; creación de paquetes y comparación reproducible.
No se ejecutaron estas comprobaciones al implementar P3.
