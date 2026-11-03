# TODO de refactorización

Actualizado: 2026-10-28. Avance estimado por alcance en REFACTOR_PROGRESS.md. Arquitectura: agente nativo → RabbitMQ → servidor
Rust → PostgreSQL/TimescaleDB; dashboard y servicios centrales en Docker.

## Prioridad vigente: completar implementación

Corrección de rumbo del 2026-10-27: terminar los bloques de código P1–P3 antes
de ampliar las campañas de pruebas. P4 conserva evidencia histórica y pendientes;
no es la siguiente fase de trabajo mientras falte implementar el bloque correspondiente.
Solo realizar comprobaciones focalizadas necesarias para el cambio (contrato,
compilación o integridad cuando corresponda). No repetir builds de todas las
plataformas, despliegues ni recorridos completos después de cada cambio pequeño.

### Orden vigente — 2026-10-28, entrada en P4

Implementación de los pendientes P2 terminada con los límites documentados en
P2_IMPLEMENTATION_CLOSURE.md. No equivale a entrega funcional certificada.

1. P4: sintaxis y compilación final Windows/frontend; luego Ubuntu/Debian y pruebas focalizadas.
2. P4: migraciones 0009–0011, consultas, concurrencia de sesiones y políticas.
3. P4: SCM/systemd, MSI/DEB, firmas, corte/rollback, migración y purga sobre datos desechables.
4. P4: RabbitMQ/retención, respaldo/restauración, UX/accesibilidad, carga y piloto.

No publicar ni reemplazar GitHub todavía. Los límites explícitos (Wayland y mapa de
coordenadas no disponible, autostart Explorer/XDG, migración sin adivinar claves)
no se presentan como funcionalidades que las pruebas vayan a implementar.

## P0 — Conseguir un despliegue completo reproducible

**Implementación cerrada según el alcance acordado.** No hay tareas de código P0
abiertas. La comprobación con copia real sigue en P4; resolución Nginx tras
recreación validada el 2026-10-25. Las comprobaciones sin evidencia no se consideran realizadas ni se certifica el estado actual para producción.
La evidencia histórica solo corresponde a las revisiones ensayadas en ese momento.

- [x] Corregir compilación: Rust 1.99.1 fijado en Docker y rust-toolchain.toml;
  Cargo.lock incluido y compilación con --locked.
- [x] Unificar el esquema del servidor en migraciones SQLx versionadas,
  transaccionales e incluidas en el binario; retirar el bootstrap SQL de Compose.
- [x] Diseñar conversiones explícitas y claves compatibles en HYPERTABLE_DESIGN.md.
  Decisión P0: no convertir automáticamente; nuevas tablas PostgreSQL normales,
  hypertables/políticas existentes conservadas. Implementación optativa en P1.
- [x] Alinear `/api`, proxy Nginx y proxy Vite; conservar URLs antiguas del agente.
- [x] Corregir parámetros de recursos, alertas y ACK para Axum 0.7.
- [x] Publicar eventos sintéticos en RabbitMQ, consumirlos y consultar CPU/RAM
  desde el mismo origen del dashboard; un reenvío no duplica la muestra.
- [x] Utilizar npm ci para el dashboard.
- [x] Fijar por digest las imágenes del despliegue y bases de los Dockerfiles.
- [x] Actualizar dependencias frontend: npm audit sin vulnerabilidades y builds
  TypeScript/Vite correctos, tanto localmente como en Docker.
- [x] Reiniciar PostgreSQL y servidor: la muestra anterior permanece y entran
  eventos nuevos después del reinicio.
- [x] RabbitMQ con hostname estable; /ready exige consumidores activos y DB accesible.
  Para volúmenes antiguos, conservar su hostname original (P0_VALIDATION.md).
- [x] Aplicar autorización central a las rutas de datos y ambos alias HTTP;
  JWT de administrador separado de x-agent-token, JWT vencidos rechazados.
  Matriz y excepciones explícitas en API_AUTHORIZATION.md; pruebas aprobadas.
- [x] Actualizar SQLx a 0.8.6: compilación sin el aviso de incompatibilidad futura.
  SQLite del agente alineado a rusqlite 0.32.1; caché verificada en Windows.
- [x] Retirar sqlx-cli de dependencias de ejecución y regenerar Cargo.lock.

## P1 — Datos útiles y entrega confiable

### Implementado en código; pendiente de validación integrada

- [x] API y panel de resúmenes diarios con períodos UTC, origen detalle/resumen/mixto
  y promedios por muestra; no se presentan como promedios ponderados por duración.
- [x] Limpieza histórica configurable por lotes para actividad (30 días), USB/WiFi
  (90), auditoría (180) y alertas resueltas (180 desde resolución).
  Desactivada por defecto con HISTORY_RETENTION_ENABLED=false; 0 desactiva cada tarea.
- [x] Excluir alertas abiertas y alertas sin fecha de resolución de la limpieza.
- [x] Registrar resultados por tarea: intento, éxito, fallo, filas y backlog acotado.
- [x] API protegida /maintenance/status y panel de mantenimiento en Métricas.
  Tamaño de base completa, relaciones físicas paginadas y totales por hypertable implementados.
- [x] Retirar tendencias de pulsaciones/clics calculadas con porcentajes ficticios.
- [x] Conservación de seguridad con investigaciones por equipo/período UTC: API de apertura/listado/liberación con motivo, exclusión de investigaciones abiertas y borrado por lotes opt-in.
  SECURITY_EVENT_RETENTION_DAYS=0 mantiene la limpieza desactivada.
- [x] Panel de investigaciones: apertura, listado paginado, filtros, liberación con motivo y atribución al usuario autenticado.
- [x] Autorización por roles viewer/operator/admin y cuentas Argon2id en archivo administrado; revocación/reducción de rol aplicada a tokens existentes.
  Override Compose opcional; consulta de cuentas/roles en P3, cambios mediante archivo protegido y comprobación de permisos en P4.

### Implementación restante P1 (validación centralizada en P4)

- [x] Deduplicación y escrituras en una transacción; ACK tras commit y errores propagados.
- [x] Tres reintentos con publisher confirms y cola durable .dead; JSON inválido conservado.
  Integraciones reales de rollback, concurrencia y replay aprobadas; ver TELEMETRY_DELIVERY.md.
- [x] Estado y aviso de DLQ en dashboard, reenvío autenticado por lotes de hasta 20, motivo/auditoría, exclusión concurrente y publisher confirms antes de ACK.
  Mensajes inválidos/vencidos detienen el lote y permanecen en DLQ; sin purga automática.
  La prueba de corte entre confirmación y ACK pertenece a P4.
- [x] Inventario actual y cambios por fuente: dpkg y HKLM 32/64; hash canónico, orden temporal y bajas solo con escaneo completo.
  Sin snapshots históricos redundantes en DB. Fuentes adicionales y migración de legado no autoritativo pendientes; ver INVENTORY_STATE.md.
- [x] Ventana global de reenvío de 30 días/120 s de adelanto; deduplicación retenida al menos 32 días, limpieza acotada opt-in. Ver REPLAY_POLICY.md.
- [x] Tamaños por relación física (tablas/chunks), índices y estimación de filas; listado acotado con truncamiento explícito.
- [x] Totales por hypertable mediante Timescale y exploración paginada de relaciones físicas en API/dashboard.
- [x] Exponer capacidad configurada de workers PostgreSQL/Timescale en API y panel.
  El diagnóstico del fallo observado de columnstore requiere ejecución del entorno y queda en P4, sin darlo por resuelto.
- [x] Decisión de conservación inicial: detalle acotado + resúmenes diarios; no generar archivos Parquet sin un consumidor definido.
  Si se solicita archivo externo, implementar manifiesto/checksum y lectura verificada antes de eliminar originales.

## P2 — Agente Windows, Ubuntu y Debian

### Implementado en código (cargo check Windows aprobado; validación funcional en P4)

- [x] AGENT_MODE=service/interactive/session explícito, con detección heredada si se omite.
- [x] Rutas configurables para configuración, identidad/caché y logs persistentes.
- [x] Fallar ante caché inaccesible o identidad corrupta; retirar fallback a memoria.
- [x] Omitir hooks y tareas de escritorio en servicio/Wayland/sin escritorio.
  Reportar elegibilidad y disponibilidad de entrada en salud/telemetría.
- [x] SIGTERM/SIGINT en Unix; Stop/Shutdown no bloqueantes en Windows.
- [x] Informar fallo del agente al SCM; retirar activación de anti-kill y WER.
- [x] Plantilla systemd con usuario dedicado, límites de reinicio y parada administrativa.
- [x] Scripts para instalación inicial Linux y registro Windows, sin arrancar el agente.

### Implementación completada — validación pendiente P4

- [x] Separación inicial service/session: sesión sin RabbitMQ, comandos, actualizador
  ni recolectores de sistema; identidad obtenida del servicio y outbox propia.
- [x] IPC loopback cifrado/autenticado, rutas permitidas, límites y ACK tras SQLite.
- [x] Encolar antes de publicar, conservar event_id, retirar descarte FIFO de pendientes
  y reemplazo automático de caché; cierre de tareas supervisadas y flock Unix.
- [x] Plantillas de configuración y sesión Linux/Windows; no instaladas ni activadas.
- [x] Políticas completas servidor → servicio → sesión, validación y persistencia atómica.
- [x] Activación/pausa dinámica de recolectores supervisados y umbral de inactividad configurable.
- [x] Enrolamiento inicial de un usuario en Linux/Windows: configuración separada,
  clave preservada, permisos restrictivos y marcador de operación; scripts no ejecutados.
- [x] Scripts Linux/Windows de activación, revocación, re-enrolamiento del mismo usuario, reparación recuperable y rotación IPC preservando la clave de la cola.
  Journal protegido y bloqueo de arranque del agente mientras esté pendiente; no equivalen a transacción multiarchivo. Ver SESSION_LIFECYCLE.md.
- [x] Claves de caché personalizadas preservadas literalmente; acción migrate para reconciliar transporte heredado sin journal, manteniendo identidad/cola y dejando procesos detenidos.
- [x] Registro/retirada de autostart por usuario: unidad systemd condicionada por UID y tarea Windows interactiva con reinicio; preservan configuración y pendientes.
- [x] Autostart por escritorio integrado: HKLM Run con dispatcher por SID en MSI, XDG Autostart por UID en DEB; reintento acotado y flags administrativos.
- [x] Diagnóstico de instancias de sesión → IPC → salud del servicio → servidor → dashboard, con antigüedad, pausa, entrada, cola y coincidencia de política. Sin historial adicional; ver SESSION_HEALTH.md.
- [x] Reacción de wrappers a disponibilidad nativa tardía sin esperar cambio de política.
- [x] Multiusuario/RDP: registro IPC por principal, enrolamiento/rotación/revocación recuperables, cola compartida por principal y eventos por ejecución; tabla/API/panel separado para no multiplicar horas de equipo (0011).
- [x] Lecturas comunes CPU/RAM/disco/red/uptime e inventario básico del sistema mediante sysinfo;
  primer intervalo sin dato, tasas por interfaz y unidades explícitas. Validación funcional en P4.
- [x] Diagnóstico por adaptador y captura en salud de sesiones: errores USB/WiFi/osquery/ventanas, estados de recursos y entrada. Cuadrícula de coordenadas no disponible explícitamente, sin datos ficticios.
  Inventario de software por estado/cambios permanece en P1.
- [x] Persistir estado actual de recursos/salud/capacidades y visualizarlo en detalle/flota;
  estado por recolector según contexto/política/supervisión. Ver TELEMETRY_CONTRACT.md.
- [x] Escritura atómica de identidad del servicio y sesión; exclusión de instancia existente.
- [x] Migración explícita de rutas/datos: plan, hashes, copia a destino nuevo, identidad/colas/sidecars preservados y permisos originales. Sin sobrescribir ni inferir claves faltantes; activación mediante AGENT_DATA_DIR.
- [x] Supervisor rechaza registro tras Stop y espera wrappers; salud, comandos y políticas supervisados.
- [x] IPC cancela conexiones hijas al cerrar listener; actualización/estado se cancelan al salir.
- [x] Deshabilitar flush_cache heredado que borraba pendientes sin confirmación.
- [x] Web cancelable y cierre cooperativo de hooks/tray Windows implementados.
- [x] Aislar rdev Linux en proceso hijo supervisado, terminado y recogido al cerrar; muerte con el padre. Retirar diálogo modal del tray Windows.
  Disponibilidad/errores del adaptador de entrada expuestos; validación nativa permanece en P4.
- [x] Generador DEB y definición/generador MSI desde artefactos locales; accesos a Mi equipo y scripts administrativos incluidos.
- [x] Verificación de actualización firmada: OpenSSL con clave aprovisionada en Linux y Authenticode con publisher fijado en Windows.
- [x] Coordinador MSI install/repair/remove con journal de sesiones, exclusión mutua, recuperación fuera de INSTALLFOLDER y restauración de autostart tras rollback. Firma/paquetes reales pendientes P4; no se afirma atomicidad multiarchivo.
- [x] Scripts locales de actualización con hash/rollback y eliminación del servicio conservando datos.
- [x] Reparación/reinstalación de servicios administrados: Linux con artefacto/hash y rutas existentes; Windows -Repair conserva ruta de binario/configuración y reconfigura recuperación.
- [x] Retirada del autostart y revocación de sesión sin borrar identidad/cola.
- [x] Journal de actualización binaria y recuperación explícita del ejecutable anterior por hash, conservando configuración/colas.
- [x] Purga explícita desde plan sin recursión, con rechazo de enlaces/cambios y protección de eventos pendientes; selección adicional por nombre. Herramienta incluida en ambos paquetes, sin ejecutarla aquí.

### Vista personal solicitada — implementación, pendiente de P4

- [x] Interfaz local Mi equipo: CPU/RAM/disco/red/uptime, pendientes y componentes, solo lectura y solo loopback.
- [x] Gráficas acotadas en memoria, antigüedad explícita y estados sin datos; no consulta otros equipos.
- [x] Accesos de escritorio previstos en paquetes y tray de sesión Windows.
- [x] Retirar "flota" de textos visibles del dashboard; usar Equipos/Equipos de oficina.

## P3 — Dashboard y entrega

### Renovación UX implementada en código (validación en P4)

- [x] Sistema visual claro: tipografía del sistema, superficies neutras, estados
  discretos, espaciado y componentes compartidos; sin fuentes externas.
- [x] Navegación por áreas, inventario/USB accesibles y menú móvil con diálogo nativo.
- [x] Login, pantalla de recuperación y marca Monitor consistentes.
- [x] Equipos con búsqueda, filtros y orden en URL, paginación y enlaces por equipo.
- [x] Conectados/desconectados/señal atrasada separados; errores parciales visibles.
- [x] Fecha de consulta y refresco sin borrar la tabla durante actualizaciones.
- [x] Alertas y picos diarios en paneles distintos; no confundir picos con consumo actual.
- [x] Administrar nombre y distinguir solicitud encolada de ejecución confirmada.
- [x] Retirar autodesinstalación remota del dashboard y rechazar comandos update/uninstall en API; mostrar el flujo de scripts administrativos.
- [x] Alertas abiertas/resueltas, búsqueda, severidad y confirmación de resolución.
- [x] Métricas por equipo/fecha con unidades y resúmenes diarios; retirar consulta
  horaria global inválida (la API exige device_id).
- [x] Inventario, USB y WiFi con tablas compartidas, búsqueda y paginación de registros
  consultados; errores distintos de resultados vacíos, límites de consulta visibles.
- [x] Detalle adaptable, solicitudes obsoletas ignoradas y recuperación de errores.
- [x] Base de accesibilidad: enlaces reales, etiquetas, foco visible, salto al contenido,
  diálogos nativos y preferencia de movimiento reducido.

### Cierre de implementación — 2026-10-28

- [x] Estado degradado informado por el agente, separado de conectividad/alertas; antigüedad y error explícitos.
- [x] Grupos por equipo, columnas recordadas en navegador y búsqueda/filtros/paginación del servidor (20 por página).
- [x] Capacidades y disponibilidad de métricas actuales: null distinto de cero, antigüedad y diagnóstico de supervisión.
- [x] Presentación de calidad/error de captura separada de supervisión, con fecha y estado desconocido explícito; USB/WiFi/inventario/aplicaciones instrumentados.
  La cobertura de errores nativos restantes y consolidación de sesiones sigue abierta en P2.
- [x] Contrato histórico de entrada nullable y resúmenes con muestras disponibles/no disponibles/desconocidas; legado no acredita disponibilidad. Migración 0010 preparada.
- [x] Guía de incorporación, políticas globales/por equipo con revisión/auditoría, herencia y confirmación por huella reportada; versiones observadas/objetivo.
- [x] Consulta administrativa de cuentas, roles y desactivaciones sin hashes; altas/cambios siguen en AUTH_USERS_FILE montado solo lectura.
  No incluye enrolamiento remoto, edición web de credenciales ni distribución automática de binarios.
- [x] Runbook DASHBOARD_OPERATIONS.md y generador release-manifest.py con commit, hashes de entradas/artefactos y entorno exacto declarado.
  La reproducción de binarios, construcción, firma y distribución pertenecen a P4.

## P4 — Pruebas, compilación y validación de entrega

Fase retomada el 2026-10-24; prioridad ajustada el 2026-10-27: campañas integrales
diferidas hasta completar el bloque de implementación. Resultados de esta
primera tanda en P4_VALIDATION.md; las comprobaciones sin evidencia siguen pendientes. Las casillas de implementación de P0–P3 no significan
validación ni autorización para desplegar los cambios nuevos.

### Primera tanda P4 — 2026-10-24

- [x] Cargo check del agente en Windows con --locked; una advertencia de código sin uso.
- [x] Siete pruebas de almacenamiento PGlite aprobadas; no equivalen a Timescale real.
- [x] Sintaxis Python del enrolador Linux y parser de los tres scripts PowerShell.
- [ ] Validar ejecución, permisos y recuperación de los scripts en entornos aislados.

- [x] Servidor Windows: 7 pruebas aprobadas, 0 fallidas; migración de integración
  omitida explícitamente por requerir TimescaleDB y esquema heredado.

### Integración y recorrido parcial — 2026-10-25

- [x] Imágenes actuales de servidor y dashboard compiladas y aplicadas al stack aislado.
- [x] Migraciones fresh/legacy actuales sobre TimescaleDB con 100.000 filas comprimidas.
- [x] Smoke HTTP actual: ingestión RabbitMQ, duplicado, auth, métricas diarias y mantenimiento.
- [x] Recorrido en vista estrecha: login, menú, búsqueda URL, estado vacío, limpiar filtros,
  diálogo Escape/retorno de foco, métricas diarias, mantenimiento y logout.
- [x] API detenida/restaurada: aviso visible y recuperación tras Actualizar.
- [x] Corregir desbordamiento de etiqueta oculta de tabla; verificado con tabla cargada.
- [x] Código del login restablece el título del documento; recorrido visual pendiente.

### Validación UX pendiente

- [x] Compilar TypeScript/Vite tras la renovación (2026-10-24).
- [x] Corregir lint: análisis completo aprobado sin errores ni advertencias
  el 2026-10-25; build TypeScript/Vite también aprobado tras los cambios.
- [ ] Revisar visualmente escritorio, tablet y móvil; tablas, gráficos y diálogos.
- [ ] Recorrer login, navegación, filtros URL, paginación, detalle y cierre de sesión.
- [ ] Probar API lenta/caída, errores parciales, listas vacías y datos atrasados.
- [ ] Probar permisos, resolución de alertas y solicitudes de administración en entorno aislado.
- [ ] Auditar contraste, teclado, foco de diálogos y lector de pantalla.
- [ ] Verificar fechas locales/UTC y límites de resúmenes diarios.

### Pendientes trasladados

- [ ] (P0) Validación externa: repetir sobre una copia representativa del volumen real.
  Pendiente por falta de copia disponible, confirmado por el usuario.
- [x] (P0) Nginx recupera la API tras recreación con IP distinta, sin reiniciar
  dashboard (2026-10-25; tests/nginx_recreation.py).
- [ ] (P1) Validar conjuntamente el mantenimiento histórico completo antes de activarlo.
  Ya existen pruebas parciales registradas; no repetirlas sin cambios relevantes.
- [ ] (P1) Ensayar reintentos, concurrencia, alertas reabiertas y Timescale comprimido;
  revisar convivencia con políticas de retención antiguas antes de habilitar.
- [ ] (P1) Verificar ACK después del commit y confirmaciones de publicación RabbitMQ.
- [ ] (P1) Probar deduplicación, reconexión, recuperación offline y cola de errores.
- [ ] (P1) Probar compatibilidad con hypertables Timescale comprimidas.
- [ ] (P1) Probar backup y restauración (independientes del archivo histórico).
- [ ] (P2) CI y pruebas de ciclo de vida en los tres sistemas operativos.
- [ ] (P3) Piloto, pruebas de carga y validación de releases reproducibles.

### Cobertura adicional P2

- [ ] Enrolamiento Linux/Windows: permisos efectivos, usuarios no autorizados y secretos.
- [ ] Interrupción/repetición de enrolamiento, symlinks/reparse y conservación de claves/colas.


- [ ] Políticas: defaults/restauración, pausa/reanudación, cambio de intervalo y alta tardía.
- [ ] Persistencia atómica Windows/Linux, falta de espacio/permisos y snapshots corruptos.
- [ ] Política offline, identidad de respuesta y eventos diferidos sin ACK ni eliminación.


- [ ] IPC: autenticación/direcciones/replay, límites, respuesta perdida y conservación del ID.
- [ ] Cola llena/corrupción: sin borrar pendientes ni confirmar antes del commit SQLite.
- [ ] Servicio+sesión: un único productor de recursos, misma identidad y cambios de UUID rechazados.
- [ ] Reinicios, concesión de sesión y límites del modo de usuario único.


- [x] Compilar agente en Windows, Ubuntu 24.04 y Debian 12; dependencias X11/Wayland/EGL resueltas para las revisiones registradas.
- [ ] Validar Ubuntu 22.04 y repetir matriz final al completar el bloque de código.
- [ ] Validar modo servicio sin escritorio y modo interactivo X11/Wayland/Windows.
- [ ] Comprobar que entrada no disponible no produce actividad ficticia.
- [ ] Probar SCM/systemd: caída, límite de reinicios, Stop, Shutdown, SIGTERM y reboot.
- [ ] Probar identidad corrupta, disco lleno/permisos, caché persistente y reinicio offline.
- [ ] Validar instalación/actualización/rollback/desinstalación MSI/DEB y firmas.
- [ ] Confirmar permisos del usuario Linux dedicado y disponibilidad de cada recolector.
- [ ] Comprobar que separación servicio/sesión no duplica métricas ni identidades.
- [ ] Validar límites de recursos, carga sostenida y migración de instalaciones heredadas.

### Comprobaciones completadas anteriormente (no cubren los cambios nuevos)

- [x] (P0) Probar en TimescaleDB una base vacía y una base con el esquema antiguo y
  datos: conservación de actividad y migración única con arranques simultáneos.
  Suite Rust actual: 7 aprobadas, 0 fallidas (incluye integración de migraciones).
- [x] (P0) Validar actualización sintética con 100.000 registros comprimidos:
  recuentos/sumas conservados, bloqueo limitado a 5 s, rollback y arranque concurrente.
  Duración local: ~0,5 s base vacía y ~10 s actualización (no es un SLA).
- [x] (P0) Completar pruebas Rust: 6 unitarias y 1 integración de migraciones aprobadas.
  Incluyen JWT, contraseñas, autorización de rutas y disponibilidad inicial.
  Registros: p0-migration-validation.log y p0-readiness-tests.log.
- [x] (P0) Verificar login, listado de equipos, métricas y consultas de alertas
  de extremo a extremo con telemetría sintética (no incluye generar/resolver alertas).
- [x] (P0) Verificar desde Windows el publicador y caché reales del agente: entrega
  AMQP confirmada y muestra consultada por la API; rechazo de mensajes sin ruta.
  Probado sin instalar el servicio ni activar recolectores (validación completa en P4).
- [x] (P0) Probar recuperación: caída real del proceso servidor bajo init, corte del
  broker de 90 s, 26 muestras pendientes y 36/36 muestras recibidas tras reconectar.
  Un mensaje durable sobrevivió a la recreación del contenedor RabbitMQ.

### Evidencia histórica anterior a esta reorganización

- [x] Docker accesible y pruebas en recursos aislados.
- [x] Compactación transaccional de recursos y actividad: detalle 14 días,
  resúmenes diarios 365 días, configurable.
- [x] Cuatro pruebas PostgreSQL WASM: límites, reintentos, datos tardíos y rollback.
- [x] Integración real en PostgreSQL 16 Docker: lotes, reintentos y rollback
  provocado por una restricción; no se pierde el registro original.
- [x] Mantenimiento periódico: múltiples ejecuciones confirmadas en Docker.
- [x] Exclusión por advisory lock: 8 ejecuciones omitidas mientras otra conexión
  mantenía el bloqueo.
- [x] Infraestructura Compose nueva: PostgreSQL 15/TimescaleDB 2.28.3,
  RabbitMQ y Redis saludables; migraciones de inicialización ejecutadas.
- [x] Imagen del dashboard compilada y HTTP servido por Nginx.
- [x] Configuración Compose y sintaxis de scripts verificadas.
- [x] Políticas de reinicio de contenedores configuradas (recuperación por fallo
  del stack completo aún no ensayada).


### Continuación puntos 1–2 — 2026-10-26

- [x] Servidor: 7 pruebas locales aprobadas; 2 integraciones nuevas aprobadas en DB/vhost temporales.
- [x] Windows: 9 pruebas focalizadas de supervisor, políticas dinámicas/persistentes, caché, IPC y escritura atómica aprobadas.
- [x] Debian 12: compilado el binario de pruebas; 10 pruebas focalizadas aprobadas con dependencias Wayland/EGL.
- [x] Ubuntu 24.04: compilación con caché propia y 10 pruebas focalizadas aprobadas en contenedor.
- [ ] Ubuntu 22.04 y matriz remota de CI: workflow preparado, sin ejecución remota.
- [x] SIGTERM: prueba Unix aprobada en Debian; recepción como parada administrativa.
- [ ] Validar servicio instalado, parada y recuperación real en SCM/systemd, y cierre de hooks/tray/web nativos.
- [x] Servidor actualizado en stack aislado; smoke RabbitMQ/DB/API/Nginx aprobado.
- [x] Web local supervisada/cancelable, token exacto, límites de cuerpo/tiempo y bind loopback por defecto.
- [x] Hooks y bandeja Windows con cierre cooperativo y espera acotada; compilación y prueba del mecanismo aprobadas.
- [x] Proceso real en Debian/Ubuntu: SIGTERM con salida 0, reinicio tras SIGKILL, identidad y pendientes conservados; recursos temporales eliminados. No equivale a systemd/SCM.


### Ciclo administrativo — 2026-10-27

Implementados scripts locales Linux/Windows de actualización con hash y rollback
ante fallo de arranque, exclusión mutua y desinstalación conservando datos. Retirados
autoactualizador del repositorio antiguo y autodesinstalación por batch destructivo.
Pruebas aprobadas con filesystem real y gestores de servicio simulados: Windows,
Debian y Ubuntu. Detalles y comandos en PLATFORM_LIFECYCLE.md. Pendientes servicios
reales, MSI/DEB, firmas, reparación/reinstalación, sesiones y recuperación ante corte
de energía durante una actualización. Sin cambios publicados en GitHub.

### Implementación métricas y salud — 2026-10-27

- [x] Contrato actual agente → RabbitMQ → estado persistido → API → dashboard.
- [x] Correcciones: RAM bytes/MiB, salud con consumidor y muestreo inicial supervisado.
- [ ] P4: migración, flujo real y revisión visual del nuevo bloque; no desplegado aún.
- Siguiente implementación: inventario por cambios y mantenimiento P1.

### Inventario y mantenimiento P1 — 2026-10-27

- [x] Estado/cambios por fuente, API paginada y búsqueda en servidor, página y panel por equipo.
- [x] Retención configurable de cambios y desglose de tamaños físicos en mantenimiento.
- [ ] P4: migración 0005 y comportamiento real del nuevo flujo; no desplegado.
- Siguiente código P1: ventana de reenvío/deduplicación, conservación de seguridad y operación de DLQ.

### P4 — Pendientes de los últimos bloques de implementación

- [ ] Autorización viewer/operator/admin, archivo de cuentas, revocación y configuración inválida.
- [ ] Diagnosticar el fallo de job columnstore observado y capacidad efectiva de workers Timescale.
- [ ] Adaptar/verificar contratos de pruebas web anteriores: ahora solo GET, Host local y sin mutaciones.
- [ ] Vista local: permisos/origen, estados vacíos/antiguos, teclado, pantallas y tray.
- [ ] Proceso X11: parada, muerte del padre, fallo de captura y ausencia de pantalla.
- [ ] Construcción MSI/DEB, firma, instalación, actualización, eliminación y recuperación de cortes reales.

### P4 — Cierre de interfaz P3, aún sin ejecutar

- [ ] Compilar Rust/TypeScript y adaptar contratos de pruebas afectados por políticas persistidas y disponibilidad de entrada.
- [ ] Migraciones 0009/0010 en fresh/legacy/Timescale comprimido y compactación de muestras desconocidas.
- [ ] Búsqueda, grupos, paginación, columnas, filtros URL y errores parciales.
- [ ] Roles de cuentas/políticas, revisión concurrente, herencia y huella de aplicación atrasada/offline.
- [ ] Diagnósticos reales, antigüedad de capturas y adaptación móvil/accesibilidad.
- [ ] Generar manifiestos en checkout limpio y comparar dos construcciones independientes por destino.

### P4 — Diagnóstico de sesión P2, sin ejecutar

- [ ] IPC de salud: reloj/secuencia, tipos/tamaño, capacidad, expiración y compatibilidad.
- [ ] Pausa, caída/reinicio del servicio y de tarea de salud, política discordante y outbox intacta.
- [ ] Aparición tardía/pérdida de entrada nativa sin cambios de política; UI de sesiones atrasadas.

### P4 — Cierre de código P2, 2026-10-28

- [x] Primera revisión sintáctica Python/PowerShell y XML WiX bien formado (no generación de MSI).
- [x] Primera compilación Windows del workspace Rust y build TypeScript/Vite aprobados; comprobar revisión final tras ajustes posteriores.
- [ ] Validar esquema 0011, consultas por día/principal, retención de sesiones y exclusión de totales heredados.
- [ ] Dos principales y dos escritorios concurrentes del mismo principal: colas compartidas, reenvío y deduplicación.
- [ ] Enrolamiento múltiple, importación de legado, revocación, claves personalizadas y journals interrumpidos.
- [ ] Paquetes: autostart Explorer/RDP/XDG, sesiones en ejecución, rollback/cortes y recuperación del coordinador.
- [ ] Migración/purga con WAL, claves correctas/incorrectas, permisos y archivos explícitos adicionales.

## Piloto local — 2026-10-28

- Proyecto Docker independiente `monitorv32-local` iniciado: server, dashboard, RabbitMQ, Redis, PostgreSQL/TimescaleDB y mantenimiento. Dashboard: http://127.0.0.1:18080. Se preservó el entorno anterior.
- Migraciones 1–11 aplicadas; Redis PONG; compilación Linux del servidor y frontend aprobada.
- `tests/local_pilot_smoke.py`: 10 comprobaciones aprobadas, incluyendo autenticación, RabbitMQ → DB → API, búsqueda/grupos, deduplicación por sesión, métricas diarias y políticas. Evidencia privada: `.local-test/logs/smoke.json`. Datos explícitamente sintéticos.
- Agente Windows release compilado; MSI 3.4.0 x64 construido con WiX 6.0.2. ZIP privado `.local-test/Monitor-Agent-Local-Windows.zip`, con instalación, reparación y eliminación coordinadas. Sin firma de producción.
- Pendiente: ejecutar el instalador en Windows, comprobar métricas reales y vista local, reinicio/recuperación, reparación/desinstalación y escenarios multisesión/Linux. No se declara P4 completo ni se instaló el agente en el anfitrión.

## Corrección del piloto: escritorio y apariencia — 2026-10-28

- El agente nativo instalado reporta actividad y snapshots de aplicaciones en `session_events`; las pantallas anteriores consultaban exclusivamente tablas heredadas.
- Migración 0012: vistas de lectura para actividad y último snapshot por usuario, sin duplicar almacenamiento ni modificar la retención. Historial, actividad reciente, aplicaciones abiertas y rankings consultan ambas generaciones. Tiempo de aplicaciones representa tiempo por persona, no encendido del equipo.
- Selector de tema Sistema/Claro/Oscuro en acceso y cabecera; persistencia local, seguimiento del sistema y sincronización entre pestañas. Paleta compartida en páginas, tablas y gráficos.
- Pendientes generales: reinicios/recuperación, reparación/desinstalación, sesiones simultáneas y Linux. La captura de mapas de calor sigue fuera del alcance implementado.

## Corrección del detalle y tema predeterminado — 2026-10-28

- Resuelto `programs.length` sobre undefined: el cliente agrupa filas reales de la API por hora, aplicación y ventana, preservando duraciones y recuentos. Se adaptaron también los nombres de campos de historial y actividad reciente.
- Tema oscuro por defecto cuando no existe preferencia guardada; Claro y Sistema siguen disponibles.
- Validación: build TypeScript/Vite aprobado; `node tests/desktop-contract.mjs` aprobado; `tests/local_desktop_smoke.py` aprobado con agente real (201 grupos históricos, 255 filas horarias, 12 aplicaciones en ese momento).
- Navegador: equipo afectado abre sin error, muestra Actividad (201), 48 controles para desplegar ventanas y permite expandirlas. Sin errores de consola en la verificación; tema oscuro confirmado. Dashboard local reconstruido y desplegado.
- Pendiente: contadores resumidos heredados de entrada/actividad no incluyen las nuevas sesiones; consultar el panel por sesión para esos valores. Continúan pendientes las pruebas generales de recuperación, reparación/desinstalación y Linux.

## UX del detalle del equipo — 2026-10-28

- Entrada predeterminada en Resumen: identidad compacta, cuatro indicadores de recursos/salud, última actividad reportada, cinco aplicaciones con más uso y accesos directos. Retirados los contadores heredados engañosos del encabezado.
- Secciones independientes: Resumen, Actividad, Aplicaciones, Recursos, USB, WiFi y Diagnóstico. El diagnóstico y las sesiones ya no aparecen delante de la actividad.
- Actividad agrupada por aplicación, búsqueda por aplicación/título, filtro por hora, ocho aplicaciones por página y ventanas desplegables (diez iniciales, cargar más). Se conservan tiempos por persona y se distinguen del tiempo encendido.
- Recursos: métricas actuales separadas de historia; almacenamiento/red desplegables. Fechas de actividad inicializadas con el calendario local.
- Verificación: build TypeScript/Vite y contrato de agrupación aprobados. Navegador con equipo real: resumen carga, actividad limita a ocho filas y búsqueda Excel reduce a una aplicación.
- Pendiente general: consolidación de contadores de entrada por sesión, ciclo de reparación/desinstalación y recuperación, escenarios Linux/multisesión.
