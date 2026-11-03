# TODO — Preparación para producción

Actualizado: 2026-11-03. Plan vigente para Monitor: agente nativo Windows/Ubuntu/Debian, RabbitMQ, servidor Rust, PostgreSQL/TimescaleDB y dashboard Docker.

Este documento reemplaza el orden de trabajo anterior. El [TODO histórico](TODO_REFACTOR_HISTORICO_2026-10-28.md) se conserva completo como referencia, no como segunda lista activa. [REFACTOR_PROGRESS.md](REFACTOR_PROGRESS.md) y [P4_VALIDATION.md](P4_VALIDATION.md) conservan la evidencia de cambios y comprobaciones anteriores.

## Estado de partida

- Piloto limpio 0.1.0 instalado y recibiendo recursos, actividad, ventanas y contadores de sesión. Datos anteriores descartados por autorización explícita; ver REFACTOR_PROGRESS.md (2026-11-03).
- Dashboard renovado: resumen por equipo, actividad agrupada, búsqueda, paginación, secciones separadas y tema oscuro predeterminado.
- Diecisiete migraciones aplicadas sobre la instalación limpia; no certifica migración de volúmenes heredados.
- Autenticación, roles, cifrado de caché/IPC, scripts de ciclo de vida y mantenimiento tienen implementación; su presencia no equivale a validación completa.
- MSI local construido y utilizado, sin firma de producción. No redistribuir el paquete del piloto: contiene credenciales locales.
- No se dispone de una copia representativa de la base anterior. No bloquear la instalación nueva por ese motivo; no certificar migración de legado sin evidencia.
- Redis está desplegado, pero no se identificó uso funcional en el servidor revisado: decidir su necesidad antes de convertirlo en requisito operativo.
- Estado global: **piloto controlado; producción no aprobada**. No hay porcentaje de preparación fiable hasta fijar alcance y cerrar los criterios siguientes.

## Versionado y documentación permanente

Agente en desarrollo: **0.1.0**, fuente de verdad `agent/Cargo.toml`. Seguir [VERSIONING.md](VERSIONING.md). Actualizar README, TODO y CHANGELOG junto con cambios de comportamiento, instalación, configuración o alcance. Los cambios exclusivos del dashboard no incrementan la versión del agente.

- [x] Piloto Windows 0.1.0: binario/MSI nuevos, hashes y versión reportada comprobados. Evidencia: REFACTOR_PROGRESS.md (2026-11-03). Paquete privado sin firma; DEB y release pública siguen en bloque 7.
- [ ] Para equipos que deban conservar datos, diseñar y ensayar transición 3.x a 0.1.x. En este piloto se eligió desinstalación y limpieza autorizada, sin forzar downgrade; no se certifica una migración que conserve identidad/claves/outbox.

## Reglas de ejecución y cierre

Primero completar el código y la configuración del bloque; después realizar su validación focalizada. Reservar la campaña completa para el bloque 9 sobre una versión candidata estable. No repetir toda la matriz por cada modificación pequeña.

Cada casilla cerrada debe enlazar un cambio o evidencia. Cada bloque requiere implementación, comprobación de su criterio de cierre y documentación operativa. Una prueba histórica solo cubre la revisión y el entorno indicados. Si un alcance se excluye, registrar explícitamente la decisión y el límite visible para el usuario; no marcarlo como implementado.

No publicar ni reemplazar el repositorio remoto, distribuir secretos o desplegar en producción por actualizar este TODO. Se conserva la condición acordada: funcionamiento probado antes de la publicación final.

| Orden | Bloque | Prioridad | Estado |
|---|---|---|---|
| 1 | Despliegue y configuración seguros | Bloqueante | En curso; perfil candidato y validación de arranque |
| 2 | Identidad y transporte de agentes | Bloqueante | Implementación cerrada; recorrido aislado aprobado |
| 3 | Acceso humano, sesiones y auditoría | Bloqueante | Implementación cerrada; comprobación aislada aprobada |
| 4 | Exactitud y completitud funcional | Bloqueante | Cerrado para candidato UTC 0.1; evidencia aislada |
| 5 | Privacidad, retención y restauración | Bloqueante | Parcial, falta validación integrada |
| 6 | Agente multiplataforma y recuperación | Bloqueante por plataforma | Parcial, Windows en piloto |
| 7 | Instaladores, actualización y releases | Bloqueante | Paquete local; firma pendiente |
| 8 | UX, accesibilidad y operación diaria | Necesario para aceptación | Base renovada; pendientes |
| 9 | Validación integrada y piloto de aceptación | Puerta de salida | Evidencia parcial |
| 10 | Operación y autorización de producción | Puerta final | Pendiente |

Regla de seguimiento: cerrar cada bloque con su implementación y las comprobaciones necesarias, informar la evidencia y dejar una lista concreta de lo que sigue pendiente. No presentar un avance parcial como cierre del bloque.

## 1. Despliegue y configuración seguros

- [x] Crear configuración candidata separada de desarrollo/piloto, con archivos de secretos sin valores de ejemplo. Ver PRODUCTION_DEPLOYMENT.md; no desplegada ni certificada.
- [ ] Fallar al arrancar si faltan secretos, son valores de ejemplo o la configuración de producción es incoherente; corregir ejemplos inseguros y variables obsoletas.
- [ ] Exponer únicamente los accesos necesarios: dashboard/API por proxy; PostgreSQL y Redis internos; RabbitMQ limitado a la red de agentes y administración restringida.
- [ ] Definir redes, reglas de firewall y accesos administrativos, incluida la ausencia de rutas que eludan el proxy.
- [ ] Separar cuentas de aplicación, migración y mantenimiento en PostgreSQL y aplicar privilegios mínimos.
- [ ] Gestionar secretos mediante archivos/mecanismo protegido; excluirlos de imágenes, repositorio, logs, manifiestos y paquetes genéricos. Documentar rotación.
- [ ] Reducir privilegios de contenedores, capacidades y escrituras; definir límites de recursos, rotación de logs y arranque del motor Docker en el anfitrión.
- [ ] Revisar imágenes/dependencias soportadas, fijar versiones/digests y definir política de parches y escaneo de vulnerabilidades.
- [ ] Decidir y documentar el uso de Redis; retirarlo del perfil de producción si no aporta una función, o configurar autenticación, ACL y persistencia según su uso real.

**Cierre:** despliegue reproducible desde configuración nueva, sin credenciales predeterminadas ni exposición de servicios internos, con inventario de puertos, secretos y privilegios documentado.

Avance 2026-10-29: lectura de secretos desde archivos, validaciones de arranque productivo, cuentas administradas obligatorias, CORS del mismo origen, overlay aislado y generador Linux sin sobreescritura. Incremento de tres puntos: roles DB separados con migrador independiente; servidor/dashboard/mantenimiento sin root; HTTPS/AMQPS configurados en el candidato. Quedan abiertos el arranque completo Linux, certificados reales/renovación, acceso remoto e identidad de agentes, escaneo y validación integrada; el bloque no está cerrado. Ver PRODUCTION_DEPLOYMENT.md.

Evidencia acotada de estos tres puntos: configuración Rust (3 casos), Compose renderizado, doce migraciones y diez trabajos de mantenimiento con permisos limitados sobre DB vacía aislada, y Nginx sin root con HTTPS/cabeceras y certificado temporal aprobados. No sustituye la validación completa del perfil ni permite cerrar los criterios compuestos de este bloque.

## 2. Identidad y transporte de agentes

- [x] Configurar HTTPS y AMQPS con validación de certificados, nombres de servidor y renovación; impedir conexiones inseguras en modo producción.
- [x] Sustituir el token global compartido por una identidad autenticada y revocable por equipo; definir enrolamiento inicial de uso limitado y su recuperación.
- [x] Vincular credencial, identidad del equipo y autorización de publicación/consulta: un agente no debe suplantar otro UUID ni leer comandos o políticas ajenos.
- [x] Aplicar permisos mínimos por agente en RabbitMQ y separar publicación de administración/consumo.
- [x] Implementar rotación y revocación de credenciales sin perder identidad ni eventos pendientes; definir qué sucede con equipos offline y certificados vencidos.
- [x] Revisar límites de mensajes/peticiones, tipos, reloj, secuencia, replay y abuso de conexiones; conservar deduplicación entre reintentos.
- [x] Exigir secreto aleatorio propio para caché/IPC en producción; eliminar fallback derivado de identificadores públicos o constantes de desarrollo. Definir custodia y recuperación de claves.

Cierre de implementación 2026-10-29: identidad HTTP/AMQPS, enrolamiento de un solo uso con recuperación idempotente, permisos mínimos del broker por credencial, comprobación del UUID antes de persistir, revocación con cierre/reconciliación, renovación automática, secretos locales cifrados y bloqueo entre procesos. Gestión administrativa en Diagnóstico; herramientas de configuración y renovación TLS con rollback. README y AGENT_IDENTITY.md actualizados.

Evidencia: servidor/agente/publicador compilan; dashboard construye; recorrido aislado con DB/broker nuevos y certificados sintéticos aprobado, incluyendo pérdida de respuestas, reanudación, acceso ajeno, prohibición de consumo/configuración y revocación de conexión abierta. El piloto no fue actualizado. Las marcas de este bloque acreditan implementación y esa evidencia local: no certifican despliegue real. Certificados/emisor, operación de renovación, adopción en servicios Windows/Linux y aceptación global siguen como puertas de los bloques 6, 9 y 10.

**Cierre:** un agente válido publica solo para su equipo; uno revocado, con certificado inválido o identidad ajena es rechazado. La rotación conserva los pendientes y no abre acceso administrativo.

## 3. Acceso humano, sesiones y auditoría

- [x] Configurar cuentas individuales con Argon2id y eliminar el usuario compartido de bootstrap del uso diario.
- [x] Completar roles viewer/operator/admin por ruta y acción, incluidos alias `/api`, exportaciones, políticas, comandos, investigaciones y DLQ.
- [x] Limitar intentos de login y abuso de API, con respuestas que no revelen cuentas válidas y alertas de intentos repetidos.
- [x] Revisar almacenamiento de JWT en localStorage; implementar sesiones protegidas y revocables. Si se usan cookies, incluir HttpOnly/Secure/SameSite y protección CSRF coherente.
- [x] Restringir CORS a los orígenes autorizados; configurar CSP, protección de framing, tipo de contenido y HSTS cuando HTTPS esté operativo.
- [x] Incorporar MFA para administradores o SSO con MFA, definiendo recuperación y cuenta de emergencia protegida.
- [x] Verificar deshabilitación, reducción de rol, expiración y revocación de sesiones; configuración de cuentas inválida debe fallar de forma cerrada.
- [x] Auditar login y operaciones sensibles con actor, equipo, acción, resultado y fecha; excluir secretos y definir acceso/retención del registro.
- [x] Revisar que consultas, búsqueda, respuestas, exportaciones y logs no filtren datos o secretos fuera del permiso concedido; proteger CSV ante fórmulas ejecutables.

**Cierre:** matriz de permisos y revocación comprobada, acceso administrativo reforzado, intentos abusivos limitados y operaciones sensibles trazables.

Cierre de implementación 2026-10-30: migración 0015, cookies opacas revocables, CSRF, MFA TOTP administrativo, cuentas con generación de sesiones, límites de login/API, pantalla de sesiones, auditoría y protección CSV/CSP. Procedimientos: [ACCESS_CONTROL.md](ACCESS_CONTROL.md). Evidencia: [P4_VALIDATION.md](P4_VALIDATION.md), recorrido `tests/identity-stack.py --human-only`, vector TOTP, escape CSV, build del dashboard, Nginx HTTPS y helper Linux aislado. El piloto permanece intacto.

Las marcas acreditan implementación y comprobaciones focalizadas, no aprobación productiva. Quedan para los bloques 5, 9 y 10 la política organizativa de privacidad/retención, aceptación en navegador del despliegue completo, custodia real de MFA/emergencia y alertas externas. El alcance de permisos es una organización con lectura de todos sus equipos; no se ofrece separación por tenant/departamento.

## 4. Exactitud y completitud funcional

Cierre 2026-11-02: implementación y comprobación focalizada completas para el alcance del candidato 0.1. Evidencia y límites: [BLOCK4_CLOSURE.md](BLOCK4_CLOSURE.md). Informes por día UTC; selección de zona IANA/DST, mapas de calor y captura global Wayland quedan expresamente fuera del piloto. No se certifican instaladores ni producción.

- [x] Consolidar contadores de entrada y tiempos por sesión en resumen, métricas y exportaciones; no sumar horas-persona como tiempo encendido.
- [x] Unificar contratos de API/frontend, tipos, errores y formato de fechas para eliminar conversiones inseguras o silencios que parezcan listas vacías.
- [x] Separar claramente actual, último reporte e histórico; identificar datos atrasados y ausencia de muestra, sin reemplazarlos por cero.
- [x] Resolver snapshots de aplicaciones de sesiones cerradas/reiniciadas, sin presentar ventanas antiguas como actualmente abiertas; conservar atribución por sesión donde corresponda.
- [x] Revisar fechas locales/UTC, medianoche, zona horaria y cambios de horario; alinear consultas, resumen, exportación y retención.
- [x] Completar consistencia de inventario actual/cambios, USB, WiFi, investigaciones y alertas reabiertas; evitar duplicados y preservar evidencia asociada.
- [x] Completar políticas: herencia global/equipo, conflicto de revisión, pausa/reanudación, valores predeterminados, aplicación offline y huella confirmada.
- [x] Revisar ciclo de comandos: autorización, validación, expiración, entrega repetida, ACK y resultado; acciones permitidas explícitamente, sin ejecución arbitraria implícita.
- [x] Definir alcance de mapas de calor y Wayland: implementar lo prometido o excluirlo expresamente de la versión, sin pantallas que aparenten disponibilidad.

**Cierre:** los mismos datos producen resultados coherentes entre vistas y exportaciones; las limitaciones se muestran y ningún indicador ausente se presenta como medición real.

## 5. Privacidad, retención y restauración

- [ ] Definir datos mínimos necesarios, acceso a títulos de ventanas/rutas/usuarios y opción de excluir o redactar información sensible antes de transmitirla.
- [ ] Documentar finalidad, información a usuarios, responsables, acceso y conservación para la organización; obtener revisión de sus políticas aplicables antes del despliegue.
- [ ] Aprobar períodos de retención por recurso, actividad, sesión, USB/WiFi, inventario, auditoría y seguridad. Diferenciar detalle, resumen, archivo y backup.
- [ ] Validar mantenimiento transaccional completo: lotes, exclusión concurrente, retrasos, reintentos, datos tardíos, alertas reabiertas y retención de deduplicación.
- [ ] Resolver el incidente de columnstore/workers Timescale y comprobar compatibilidad con chunks comprimidos antes de activar políticas afectadas.
- [ ] Decidir si se necesita archivo histórico adicional; si se implementa, definir cifrado, verificación, catálogo, recuperación y vencimiento. No conservar archivos indefinidamente por defecto.
- [ ] Automatizar backups cifrados y separados del anfitrión, incluyendo configuración y material de recuperación necesario; definir RPO/RTO y alertas de fallo.
- [ ] Restaurar un backup en un entorno limpio y comprobar datos, identidades, sesiones, claves, políticas y funcionamiento; medir tiempos reales.
- [ ] Activar eliminación histórica solo tras comprobar límites y recuperación; eliminar también copias/archivos conforme al plan aprobado.

**Cierre:** retención medida y acotada, restauración demostrada dentro de objetivos acordados, sin confundir compactación o archivo con respaldo.

## 6. Agente multiplataforma y recuperación

- [ ] Fijar versiones soportadas de Windows, Ubuntu y Debian y matriz de escritorio/servicio, X11/Wayland y RDP; incluir Ubuntu 22.04 mientras siga dentro del alcance acordado.
- [ ] Verificar SCM/systemd: arranque tras reboot, recuperación ante caída, límites de reinicio, parada intencional, shutdown/SIGTERM y cierre de hooks/tray/web/procesos hijos. Evidencia parcial Windows: recuperación automática de SCM en 16 s tras caída, 2026-11-03.
- [ ] Validar permisos de servicio y usuario dedicado Linux, separación servicio/sesión y captura con usuario interactivo limitado.
- [ ] Comprobar enrolamiento múltiple, dos usuarios y dos escritorios del mismo usuario, autostart Explorer/RDP/XDG y ausencia de duplicación de identidad/recursos. Piloto de un usuario: diagnóstico con principal y capturas confirmado; el vacío inicial precedía a la consolidación de cinco minutos. Multiusuario/RDP sigue pendiente.
- [ ] Probar IPC autenticado: claves, origen, replay, reloj/secuencia, límites, expiración y pérdida de respuesta; conservar ID y pendientes.
- [ ] Probar desconexión prolongada, caída de RabbitMQ/server, reconexión y reenvío; SQLite WAL y cola llena/corrupta no deben descartar eventos pendientes silenciosamente. Evidencia parcial Windows: corte RabbitMQ 75 s, 11 pendientes, drenaje a cero e identidad conservada, 2026-11-03.
- [ ] Probar disco lleno, permisos insuficientes, configuración/identidad corruptas y apagado durante escritura; mantener persistencia recuperable y errores comprensibles.
- [ ] Comprobar captura nativa al aparecer/desaparecer el escritorio o dispositivo de entrada, pausa/reanudación y proceso X11 aislado; ausencia de captura no genera actividad ficticia.
- [ ] Medir CPU, RAM, disco, red y tamaño de caché del agente; fijar límites y comportamiento al alcanzarlos.
- [ ] Verificar vista local: solo loopback/lectura, origen y permisos efectivos, estados antiguos/vacíos y ausencia de datos de otros equipos.

**Cierre:** cada plataforma anunciada tiene evidencia de instalación nativa, recolección, recuperación y parada correcta. Una plataforma sin esa evidencia no se anuncia como certificada.

## 7. Instaladores, actualización y releases

- [ ] Generar MSI/DEB desde checkout limpio con versiones, dependencias, manifiestos y hashes; comparar construcciones independientes y documentar cualquier diferencia.
- [ ] Firmar paquetes/binarios Windows y canal de distribución Linux; custodiar las claves de firma y fijar la confianza del publicador.
- [ ] Separar paquete genérico del enrolamiento: no incrustar secretos compartidos ni credenciales del piloto en instaladores redistribuibles.
- [ ] Validar instalación, reparación, actualización, downgrade rechazado, rollback y desinstalación con sesiones activas y autostart coordinado.
- [ ] Ensayar cortes en cada etapa, journals interrumpidos y recuperación; conservar identidad, claves, configuración y outbox.
- [ ] Validar importación de legado, claves personalizadas, symlinks/reparse/hardlinks y migración con WAL; no adivinar claves ni sobrescribir datos ajenos.
- [ ] Validar purga explícita, archivos adicionales y advertencia por pendientes; desinstalación normal debe preservar lo documentado.
- [ ] Definir canal estable y piloto, actualización gradual y reversión. Mantener autoactualización remota deshabilitada hasta completar firma, autorización y rollback seguros.
- [ ] Preparar notas de versión, requisitos, cambios incompatibles, instrucciones de incorporación y soporte; retirar artefactos privados/de prueba de la distribución.

**Cierre:** artefactos firmados y verificables, sin secretos embebidos, con ciclo de vida probado en cada destino anunciado y recuperación documentada.

## 8. UX, accesibilidad y operación diaria

- [ ] Completar resumen con contadores por sesión fiables y mensajes accionables; mantener diagnóstico técnico fuera del recorrido habitual.
- [ ] Revisar aplicaciones/inventario, USB/WiFi y diagnóstico para evitar tablas sin búsqueda/paginación o pantallas excesivamente largas.
- [ ] Preservar contexto de equipo, pestaña, fecha, búsqueda y filtros al navegar, volver o compartir un enlace autorizado.
- [ ] Aislar fallos de carga por sección: una API secundaria caída no debe impedir consultar todo el equipo. Añadir reintento local y conservación explícita de datos anteriores.
- [ ] Alinear botones y permisos con el rol sin depender de la interfaz como control de seguridad; explicar acciones rechazadas.
- [ ] Validar estados vacíos, lentos, desconectados, desactualizados y errores de exportación; evitar cifras o estados ficticios.
- [ ] Revisar teclado, foco, lector de pantalla, contraste y tablas/gráficos en temas claro/oscuro y tamaños escritorio/tablet/móvil.
- [ ] Unificar nombres, unidades, duraciones y textos de ayuda; diferenciar aplicaciones reportadas, software instalado y actividad enfocada.
- [ ] Completar ayuda operativa para incorporar equipos, resolver incidencias y consultar Mi equipo; no exponer credenciales en capturas/documentación.

**Cierre:** recorridos cotidianos completos con usuarios representativos, sin errores de renderizado ni bloqueos de navegación, accesibles y con información priorizada.

## 9. Validación integrada y piloto de aceptación

Ejecutar la campaña sobre una versión candidata identificada después de completar los bloques anteriores. Repetir solo casos afectados por cambios posteriores o fallos encontrados.

- [ ] Ejecutar CI de compilación, contratos y pruebas relevantes en las plataformas acordadas, registrando commit, artefacto y entorno.
- [ ] Validar migraciones en instalación nueva y escenarios heredados representativos, incluidas 0005 y 0009–0012, chunks comprimidos y conservación de datos.
- [ ] Cuando exista copia real autorizada, repetir migración de legado; mientras no exista, registrar esa limitación y usar fixtures representativos sin certificar el volumen real.
- [ ] Verificar confirmación RabbitMQ, ACK solo después de commit, duplicados, concurrencia, rollback y recuperación ante caída en puntos críticos.
- [ ] Ensayar DLQ/reenvío autorizado, rechazo de eventos inválidos, eventos tardíos/offline y ausencia de pérdida silenciosa.
- [ ] Ejecutar matriz de seguridad: roles, revocación, suplantación, abuso de login, inyección, XSS, exportaciones, origen local y permisos de archivos.
- [ ] Escanear dependencias, imágenes, secretos y paquetes; resolver hallazgos críticos/altos o documentar aceptación explícita con responsable y vencimiento.
- [ ] Realizar carga sostenida con cantidad objetivo de equipos e intervalos reales; medir latencia, CPU/RAM, I/O, colas, crecimiento DB y tiempo de recuperación.
- [ ] Ejecutar piloto de duración y tamaño acordados en las tres plataformas, cubriendo sesiones, política, desconexión, reinicio y ciclo de actualización.
- [ ] Registrar defectos, severidad, corrección y regresión focalizada; completar aceptación UX y restauración de backup de la versión candidata.

**Cierre:** informe único con evidencias, límites y defectos conocidos; sin bloqueantes abiertos. Definir cantidad de equipos, duración, presupuesto de recursos y objetivos antes del ensayo, no después de ver resultados.

## Preparación de publicación del código piloto

- [x] Retirar ejecutable heredado del índice y credenciales detectadas del árbol actual; añadir preflight y releases en borrador. Ver GITHUB_PUBLICATION.md.
- [ ] Identificar y revocar el token GitHub histórico; rotar credenciales antiguas de RabbitMQ si siguen activas.
- [x] Destino acordado 2026-11-03: actualizar Monitorv3/main conservando historial, sin force-push ni repositorio nuevo. Usuario autoriza subir el árbol limpio con revocación histórica aún pendiente. Ver GITHUB_PUBLICATION.md.
- [ ] Ejecutar CI remoto y revisar resultados después de autorizar la subida. Publicar código piloto no certifica producción.

## 10. Operación y autorización de producción

- [ ] Configurar métricas y alertas externas: caída de API/consumidores, cola pendiente/DLQ, espacio DB/disco, mantenimiento, backups, certificados y agentes sin señal.
- [ ] Publicar runbooks de instalación, enrolamiento, rotación/revocación, backup/restore, actualización, rollback y diagnóstico, con responsables y escalamiento.
- [ ] Definir capacidad inicial, crecimiento, infraestructura, dominio/certificados, acceso de soporte y ventanas de mantenimiento; identificar los recursos externos aún necesarios.
- [ ] Preparar checklist de corte: backup previo, configuración validada, credenciales nuevas, versión firmada y plan/umbral de reversión.
- [ ] Obtener aceptación del alcance soportado, límites de captura, políticas de acceso/retención, métricas de servicio y defectos no bloqueantes.
- [ ] Confirmar que la versión candidata coincide con los artefactos ensayados y que ningún archivo privado del piloto se publica.
- [ ] Autorizar publicación/distribución y despliegue gradual solo después de la aceptación; no borrar o reemplazar el repositorio anterior sin instrucción específica vigente.
- [ ] Supervisar la primera instalación productiva y ampliar por etapas con criterios de parada; verificar restauración y soporte disponibles.

**Cierre:** aprobación explícita de salida, artefactos identificados, alertas activas, backup recuperable y responsables operativos definidos. “Listo para producción” solo se declara entonces.

## Dependencias y decisiones por resolver durante el trabajo

- Infraestructura de destino, dominio/DNS, certificados y redes de agentes.
- Organización/usuarios, necesidad de SSO, proveedor de identidad y MFA.
- Cantidad objetivo de equipos, sistemas/versiones, capacidad y objetivos RPO/RTO.
- Política de privacidad y retención, exclusiones de captura y necesidad real de archivo histórico.
- Certificado de firma Windows y custodia de claves; canal Linux de distribución.
- Acceso a runners/VMs de prueba y eventual copia de base heredada.

Estas decisiones se solicitarán cuando bloqueen un paso concreto. Mientras tanto, se puede avanzar con configuración parametrizada, controles locales y código, sin inventar certificados, infraestructura ni aprobaciones.

## Correspondencia con el plan anterior

- P0 y migraciones pendientes → bloques 1, 5 y 9.
- P1, contratos, roles, RabbitMQ y almacenamiento → bloques 2, 3, 4, 5 y 9.
- P2, agente, IPC, enrolamiento, paquetes y recuperación → bloques 2, 6, 7 y 9.
- P3, dashboard, vista local, políticas y experiencia de uso → bloques 4 y 8.
- P4, carga, seguridad, plataformas y aceptación → bloque 9; evidencias ya existentes se reutilizan cuando correspondan.
- Entrega y operación continuada → bloque 10.

Las entradas antiguas “no desplegado” o “instalación pendiente” quedan como hechos históricos, no describen el piloto actual. Los casos todavía no comprobados siguen abiertos en este plan.
