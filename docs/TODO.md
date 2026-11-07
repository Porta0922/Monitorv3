# TODO — Ruta de cierre hacia producción

Seguimiento **2026-11-07**: PR #2 fusionado en `ba7e3b3`, con Publication preflight, Stack smoke, Storage retention y Agent lifecycle checks aprobados para su cabeza `178f16c3`. PR #1 ya estaba fusionado. Se conserva el historial. RabbitMQ 4.2 validado en entorno aislado; piloto sin actualizar. No hay certificación productiva.

Actualizado: **2026-11-06**. Estado: **piloto controlado; producción no aprobada**. Agente **0.1.0**. Este es el único plan activo. El [plan anterior completo](TODO_PRODUCCION_HISTORICO_2026-11-06.md) conserva el detalle y las evidencias para consulta, no como segunda lista de ejecución.

Base: [informe de seguridad](SECURITY_REVIEW_2026-11-06.md), [progreso](REFACTOR_PROGRESS.md) y [diagnóstico del piloto](PILOT_TROUBLESHOOTING.md). Las correcciones de seguridad del 6 de noviembre están publicadas mediante PR #2; no desplegadas en el piloto.

## Ya completado: conservar, no rehacer

- [x] Identidad individual y transporte autenticado de agentes; comprobación aislada.
- [x] Sesiones, MFA administrativo, roles, CSRF, revocación y auditoría; matriz aislada aprobada.
- [x] Renovación del dashboard y consistencia de métricas para el alcance UTC del candidato 0.1.
- [x] Endurecimiento local de Nginx/API, bloqueo de archivos, límites y registros seguros; comprobaciones focalizadas aprobadas.
- [x] Corrección de avisos npm detectados y build aprobado.
- [x] Inventario inicial de secretos, dependencias Rust e imágenes; hallazgos pendientes documentados.

Estas marcas acreditan implementación y evidencia acotada, no aceptación productiva del despliegue final.

## Orden sugerido

| Orden | Bloque | Resultado |
|---|---|---|
| 1 | Credenciales y dependencias | Riesgos conocidos resueltos y bases compatibles |
| 2 | Despliegue y alertas | Configuración reproducible y servicios protegidos |
| 3 | Privacidad, retención y recuperación | Datos acotados y respaldo recuperable |
| 4 | Agente multiplataforma | Captura, persistencia y recuperación por plataforma |
| 5 | Instaladores y actualización | Paquetes genéricos verificables y rollback |
| 6 | Seguridad y aceptación del dashboard | Frontend final comprobado |
| 7 | Candidato y piloto integrado | Evidencia de todo el sistema sobre una versión concreta |
| 8 | Publicación y salida gradual | Artefactos aprobados, soporte y reversión |

Primero implementar cada bloque y luego ejecutar sus comprobaciones focalizadas. Reservar la campaña completa para el bloque 7 y repetir después solo lo afectado por cambios. La revocación del token se atiende inmediatamente, pero no bloquea trabajo local independiente. Kubernetes queda como evolución posterior.

## 1. Credenciales y dependencias — en curso

- [ ] Confirmar revocación del token GitHub histórico de usb/agent-config.json y revisar su uso; rotar credenciales antiguas todavía activas. Requiere acceso del titular. Borrar archivos o commits no revoca tokens.
- [ ] Evaluar limpieza posterior del historial/copias accesibles; no reescribir ni hacer force-push automáticamente. Mantener el repositorio existente como destino acordado.
- [x] Migrar reqwest/hyper/h2 a una combinación compatible corregida; revisar agente, servidor y probe. Avance 2026-11-06: reqwest 0.12.28 con JSON/native-tls y sin HTTP/2 opcional; h2 0.3.27 eliminado del lockfile. Cargo check Windows y recorrido de identidad/transporte aislado aprobados; Linux aprobado por CI.
- [x] Identificar features/targets que incorporan rsa y eliminarlo si no se usa, o documentar exposición y tratamiento del aviso sin parche. Clasificación 2026-11-07: presente en Cargo.lock por sqlx-mysql opcional, sin ruta activa en `cargo tree --locked --target all -i rsa` con las funciones PostgreSQL actuales. Aviso conservado, sin excepción; reevaluar al cambiar funciones o artefactos.
- [ ] Resolver dependencias sin mantenimiento y paquete retirado del registro; revisar Windows/Linux. Avance: dotenv sustituido por dotenvy, env_logger sin uso retirado del agente y async-executor actualizado a 1.14.1. Actualización local Lapin 4.12.3 elimina los avisos de mantenimiento; build Windows y AMQPS con RabbitMQ 4.2 aprobados. CI Linux del nuevo incremento pendiente.

Seguimiento del CI: fixture Windows corregido y aprobado por el runner; prueba de origen/autenticación local ajustada al contrato vigente y aprobada en Windows. Captura Linux sustituye xcap por XCB para evitar incompatibilidad PipeWire con Ubuntu 22.04; compilación y comprobaciones Linux aprobadas en CI; aceptación con escritorio real pendiente. Auditoría tras esa sustitución: rsa sigue abierto y quedan dos avisos de mantenimiento (instant y rustls-pemfile). Los dos avisos de mantenimiento se eliminaron posteriormente con Lapin 4.12.3; rsa queda clasificado como dependencia opcional no activa. La prueba de migraciones ahora exige todo el historial embebido, conservando aserciones de integridad de datos.
- [ ] Actualizar bases y servicios Docker soportados, fijar digests y reconstruir. Revisar compatibilidad y ruta de actualización de RabbitMQ/PostgreSQL/Timescale antes de modificar volúmenes. Conservar RabbitMQ.
- [ ] Retirar dependencias sin uso demostrado. Redis retirado de Compose base/local/producción y del ensayo; contenedor del piloto detenido y eliminado, volumen conservado. No hay consumidores de Redis en el código.
- [ ] Generar inventario de componentes/SBOM y volver a escanear artefactos nuevos, registrando herramientas, fecha y versiones.

Imágenes: dashboard candidato con Nginx 1.30.6 fijado por digest, build y comprobación HTTPS aprobados, cero avisos conocidos en su escaneo. Script `scripts/security-images.ps1` genera informes privados y SBOM por ID de imagen, con umbral crítico/alto opcional. RabbitMQ 4.2 candidato también pasó el escaneo y el recorrido aislado de identidad/AMQPS; migración y restantes imágenes siguen pendientes; ver [IMAGE_SECURITY.md](IMAGE_SECURITY.md).

**Cierre:** correcciones comprobadas y hallazgos clasificados. Riesgos críticos/altos explotables bloquean producción; cualquier excepción requiere responsable, mitigación, motivo y vencimiento. No confundir avisos de escáner con explotación demostrada ni análisis con corrección.

## 2. Despliegue seguro y alertas

- [ ] Arrancar perfil productivo completo desde configuración limpia, sin defaults ni secretos en imágenes, paquetes, repositorio o logs.
- [ ] Verificar HTTPS/AMQPS, confianza, nombres, renovación y vencimiento; comprobar cookies Secure/HttpOnly/SameSite en el destino real.
- [ ] Verificar redes/firewall y ausencia de bypass del proxy; DB/Redis internos, broker limitado a agentes y administración restringida.
- [ ] Comprobar privilegios efectivos de aplicación/migración/mantenimiento DB, permisos de secretos, usuarios de contenedor, capacidades y límites de recursos.
- [ ] Confirmar debug desactivado, errores sin información sensible y rotación/retención de logs; documentar rotación de secretos y arranque Docker tras reboot.
- [ ] Configurar alertas externas por abuso, accesos denegados, API/consumidores caídos, cola/DLQ, disco/DB, mantenimiento, backups y certificados; asignar receptor y respuesta.
- [ ] Ajustar límites para oficinas detrás de NAT. Antes de múltiples réplicas, coordinar límites actualmente mantenidos por instancia.

**Cierre:** despliegue reproducible, inventario de puertos/privilegios y recepción de alertas demostrada. La configuración escrita no acredita su aplicación real.

## 3. Privacidad, retención y recuperación

- [ ] Definir mínimos de captura y exclusión/redacción de títulos, rutas y datos sensibles antes de transmitir; documentar finalidad, acceso y aviso a usuarios.
- [ ] Aprobar retención por actividad, recursos, sesiones, inventario, USB/WiFi, auditoría y seguridad; separar detalle, resumen, archivo y backup.
- [ ] Resolver compatibilidad Timescale/columnstore/workers y mantenimiento con chunks comprimidos, lotes, concurrencia, reintentos, datos tardíos y deduplicación.
- [ ] Decidir archivo histórico adicional; si hace falta, implementar cifrado, catálogo, verificación, recuperación y vencimiento.
- [ ] Automatizar backups cifrados fuera del anfitrión, incluyendo configuración y claves necesarias; fijar RPO/RTO y alertas.
- [ ] Restaurar en entorno limpio y comprobar identidades, políticas y datos; medir recuperación y verificar límites antes de activar eliminación productiva.

**Cierre:** retención acotada y restauración demostrada. Sin copia heredada representativa, comprobar instalación nueva y fixtures sin certificar una migración real.

## 4. Agente y recuperación multiplataforma

- [ ] Fijar versiones Windows/Ubuntu/Debian, incluyendo Ubuntu 22.04 mientras forme parte del alcance; matriz servicio/sesión, X11/Wayland y RDP.
- [ ] Implementar drenaje adaptativo acotado tras cortes largos, sin pérdida ni snapshots antiguos sobrescribiendo estado actual.
- [ ] Revisar IPC autenticado, claves, origen/replay, secuencias, permisos locales y separación servicio/usuario; vista local solo loopback/lectura.
- [ ] Completar recuperación SCM/systemd, reboot, parada intencional, shutdown y cierre de procesos/hooks; usuarios limitados, sesiones múltiples y autostart RDP/XDG.
- [ ] Cubrir desconexión, cola llena/corrupta, disco lleno, permisos, identidades corruptas y escritura interrumpida, sin pérdida silenciosa.
- [ ] Medir CPU/RAM/disco/red/caché y definir límites; verificar captura al aparecer/desaparecer escritorio/dispositivos, sin actividad ficticia.

**Cierre:** evidencia nativa de cada plataforma anunciada. Conservar límites visibles de Wayland y alcance UTC; no anunciar captura no implementada ni extrapolar evidencia Windows a Linux.

## 5. Instaladores y actualización segura

- [ ] Generar MSI/DEB desde checkout limpio, con versiones, manifiestos, hashes y análisis de reproducibilidad.
- [ ] Separar paquete genérico y enrolamiento; excluir credenciales del piloto y secretos compartidos.
- [ ] Configurar firma Windows y confianza del canal Linux; custodiar claves y verificar publicador.
- [ ] Validar instalación/reparación/actualización, downgrade rechazado, rollback y desinstalación con sesiones activas; conservar identidad/claves/outbox según política.
- [ ] Cubrir cortes, journals, WAL, symlinks/reparse/hardlinks, purga explícita y archivos ajenos; definir transición de legado únicamente para escenarios soportados.
- [ ] Preparar canales piloto/estable y notas. Mantener autoactualización remota deshabilitada hasta completar firma, autorización y rollback.

**Cierre:** paquetes firmados sin secretos y ciclo de vida comprobado por destino. Seguir [VERSIONING.md](VERSIONING.md); continuar serie 0.1.x durante piloto y versionar cambios del agente distribuido.

## 6. Dashboard: seguridad y aceptación

- [x] Corregir lint de AdministrationPage.tsx; reinicio del estado trasladado a la acción de actualizar. Lint global aprobado; aceptación final del frontend sigue en los puntos siguientes.
- [ ] Probar payloads XSS en títulos, nombres, tablas, búsquedas y exportaciones; verificar escape/CSP en navegador, no solo cabeceras.
- [ ] Inspeccionar assets y almacenamiento: sin API keys, secretos ni sesión accesible a JavaScript; verificar cierre/revocación y errores seguros.
- [ ] Recorrer roles, datos vacíos/atrasados/error, temas, móvil, teclado y lector de pantalla; completar aceptación humana.
- [ ] Confirmar que auditoría y pantalla de seguridad no divulgan secretos ni datos fuera del rol; conservar protección de fórmulas CSV.

**Cierre:** lint/build y pruebas focalizadas aprobados, aceptación registrada y límites explícitos. No rehacer la renovación UX ya terminada.

## 7. Candidato y piloto integrado

- [ ] Fijar commit, versiones/hashes, plataformas, cantidad de equipos, duración, recursos y criterios de éxito antes del ensayo.
- [ ] Ejecutar CI por destino y despliegue limpio con artefactos del candidato; migraciones nuevas y escenarios heredados representativos disponibles.
- [ ] Verificar recorrido agente → RabbitMQ → DB → dashboard: ACK tras commit, duplicados, concurrencia, rollback, DLQ, eventos inválidos/tardíos y recuperación.
- [ ] Ejecutar matriz final de seguridad: roles, MFA, revocación, suplantación, inyección, XSS, CSRF, límites, archivos/permisos, secretos y dependencias.
- [ ] Medir carga sostenida, latencia, recursos, crecimiento DB, colas y recuperación con parámetros representativos.
- [ ] Ejecutar piloto Windows/Ubuntu/Debian con desconexiones, reinicios, políticas, actualización, restauración y aceptación UX.
- [ ] Registrar defectos y regresión focalizada; emitir informe único con evidencia, límites y riesgos residuales.

**Cierre:** candidato identificado sin bloqueantes abiertos. No presentar resultados históricos como pruebas de binarios o imágenes nuevos.

## 8. Publicación y operación gradual

- [ ] Actualizar README, TODO, CHANGELOG y runbooks de instalación, enrolamiento, rotación, backup/restore, mantenimiento, rollback y soporte.
- [ ] Preparar publicación del código piloto: revisar diff, secretos, archivos expuestos y CI. Un commit de avance no equivale a release estable ni exige esperar a producción.
- [ ] Publicar releases con artefactos ensayados, hashes, firmas y notas; excluir material privado.
- [ ] Preparar corte con backup recuperable, credenciales nuevas, umbrales de parada/rollback, responsables y escalamiento.
- [ ] Obtener aceptación del alcance, privacidad, retención y riesgos residuales; autorizar distribución productiva.
- [ ] Desplegar gradualmente, confirmar alertas/soporte y ampliar según criterios.

**Cierre:** versión aprobada con recuperación y responsables definidos. Publicar código, actualizar el piloto y desplegar producción son acciones distintas.

## Decisiones externas

Decisión del usuario (2026-11-07): credenciales y destino real se completarán al desplegar en producción. DevOps dispone de [RELEASE_DEVOPS.md](RELEASE_DEVOPS.md), con rutas y procedimiento. Esto no impide continuar código/validaciones aisladas ni acredita aceptación del destino real.

| Recurso o decisión | Cuándo hace falta |
|---|---|
| Confirmación de revocación del token | Inmediato, bloque 1 |
| Infraestructura, dominio, certificados y redes | Destino real, bloque 2 |
| Receptor de alertas y responsables | Bloque 2 |
| Política de privacidad/retención y RPO/RTO | Bloque 3 |
| Versiones SO y acceso a equipos/VMs | Bloques 4–7 |
| Certificado Windows y claves Linux | Bloque 5 |
| Tamaño, duración y capacidad objetivo del piloto | Antes del bloque 7 |

Solicitar cada dato cuando bloquee una acción concreta; avanzar con trabajo independiente y configuración parametrizada mientras tanto.

## Regla de seguimiento

Cerrar cada bloque con implementación, evidencia y lista de pendientes. Indicar si los cambios están locales, publicados o desplegados. No marcar una casilla compuesta por completar solo una parte. Actualizar documentación junto con cambios de comportamiento.

Correspondencia: antiguos 1/2/3 → nuevos 1/2/6; antiguo 5 → nuevo 3; antiguo 6 → nuevo 4; antiguo 7 → nuevo 5; antiguos 4/8 → nuevo 6 y aceptación 7; antiguo 9 → nuevo 7; antiguo 10 → nuevo 8. El archivo histórico conserva el detalle previo.
