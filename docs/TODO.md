# TODO — Cierre hacia producción

Actualizado: **2026-11-22**. Esta lista reemplaza los estados mezclados anteriores;
el detalle histórico permanece en Git y en REFACTOR_PROGRESS.md.
Una casilla cerrada acredita solamente el alcance indicado, no producción completa.

## Estado verificable

Publicación en preparación: **0.2.5**, mismo supervisor comprobado localmente en
0.2.4. CI y existencia de release/paquetes se confirman antes de distribuir.

- Release pública **0.2.3**, fusionada en main: `6a1ef7a61e3f00f6b14a44588d2dd4a96c5a915b`.
- [x] PR #11 y #12 fusionados después de CI satisfactorio.
- [x] CI del commit: ciclo de vida Windows/Linux, preflight/seguridad navegador/backups,
  retención y stack integrado aprobados. Enlaces en [evidencias](PRODUCTION_EVIDENCE.md).
- [x] Release [agent-v0.2.3](https://github.com/Porta0922/Monitorv3/releases/tag/agent-v0.2.3)
  publicada: MSI, ZIP Windows, DEB Debian 12/Ubuntu 22.04/24.04, código y SHA256SUMS.
  Es un candidato sin firma, no una versión estable aprobada.
- [x] Piloto Windows actualizado 0.2.2 → **0.2.3** desde GitHub mediante tarea SYSTEM;
  configuración conservada, servicio Running y una sesión de escritorio activa.
- [x] Recuperación inicial del 21/11: publicador conectado tras actualización y cola
  drenada desde más de 2.100 eventos.
- [x] Instalar supervisor 0.2.4 localmente conservando configuración y sesión.
  El puerto Docker cerraba AMQP incluso con cliente independiente; recrear solo
  RabbitMQ conservando volumen recuperó el puerto y el agente reconectó solo.
- [ ] Observar estabilidad del puerto Docker y del agente 0.2.4; no declarar
  resuelta definitivamente la recurrencia por una recuperación satisfactoria.
- [x] Confirmar recepción en API del dashboard: 9 aplicaciones vigentes, última observación
  21/11 11:23:11 UTC; cola de 1–2 eventos con captura activa.

## 1. Seguridad, dependencias e imágenes

### Implementado y comprobado dentro del alcance documentado

- [x] Identidad individual, transporte autenticado, sesiones, MFA administrativo,
  roles, CSRF, revocación y auditoría; matriz aislada aprobada.
- [x] Endurecimiento Nginx/API: rutas, archivos, cabeceras, límites y errores seguros.
- [x] Actualización reqwest/hyper; eliminación de h2 vulnerable y dependencias sin uso
  identificadas (env_logger), sustitución dotenv y actualización Lapin.
- [x] Clasificación de rsa: dependencia opcional sqlx-mysql sin ruta activa PostgreSQL.
  Reevaluar al cambiar features; no equivale a eliminar el aviso del lockfile.
- [x] Retirar Redis de Compose y consumidores; volumen anterior conservado.
- [x] Automatizar escaneo de imágenes y SBOM; escanear candidatos identificados en
  [IMAGE_SECURITY.md](IMAGE_SECURITY.md). No repetirlos sin cambio de artefacto o base de avisos.
- [x] Construir candidatos corregidos de servidor, dashboard y PostgreSQL/Timescale;
  RabbitMQ 4.2 comprobado en entorno aislado.

### Pendiente

- [ ] Confirmar revocación y revisar uso del token histórico de `usb/agent-config.json`;
  rotar cualquier credencial antigua activa. No reescribir historial ni hacer force-push.
- [ ] Evaluar copias históricas accesibles después de la revocación, con decisión del titular.
- [ ] Clasificar hallazgos restantes de imágenes (incluidos dos sin severidad en PostgreSQL)
  y registrar responsable, mitigación y vencimiento de excepciones.
- [ ] Promover imágenes/digests aceptados al despliegue real; completar aceptación de la
  migración RabbitMQ/PostgreSQL/Timescale antes de modificar datos productivos.
- [ ] Inventariar y escanear los artefactos finales; retirar otros paquetes solo si se
  demuestra que no se usan. Los escaneos históricos no certifican binarios nuevos.

## 2. Configuración productiva y alertas — DevOps

- [x] Perfil productivo, secretos por archivo y guía [RELEASE_DEVOPS.md](RELEASE_DEVOPS.md).
- [x] Transporte SMTP TLS/STARTTLS y comprobaciones de cifrado/rechazo seguro.
- [x] Alertas HTTPS, rechazo de CA/redirecciones, cooldown y recuperación en ensayo aislado.
- [ ] Configurar credenciales nuevas, dominio y certificados en destino; arrancar perfil
  limpio y verificar HTTPS/AMQPS, renovación y cookies Secure/HttpOnly/SameSite.
- [ ] Verificar firewall, DB interna, acceso administrativo, privilegios DB/contenedores,
  permisos de secretos, capacidades y límites efectivos; ausencia de bypass del proxy.
- [ ] Confirmar debug desactivado, rotación de logs/secretos y arranque tras reboot.
- [ ] Sustituir contraseña habitual de correo por contraseña de aplicación autorizada o
  relay SMTP; comprobar recepción y asignar responsables. No hay entrega de correo acreditada.
- [ ] Activar alertas de abuso, API/consumidores, cola/DLQ, disco, mantenimiento, backups
  y certificados en el destino real.
- [ ] Ajustar límites para NAT; coordinar límites por instancia antes de múltiples réplicas.

## 3. Privacidad, retención y recuperación

- [x] Política de escritorio en agente/servidor/dashboard: títulos/rutas, aplicaciones
  excluidas y filtrado antes de caché/envío; [DESKTOP_PRIVACY.md](DESKTOP_PRIVACY.md).
- [x] Implementar mantenimiento, backups cifrados age y transferencia verificable rclone.
- [x] Recuperación sintética de 5.000 filas comprimidas/configuración/definiciones broker;
  migración PG15 → PG17 y Timescale 2.28.3 → 2.32.0 comprobadas.
- [x] Transferencia/reinicio/reversión de 72 mensajes sintéticos RabbitMQ, incluidos
  duplicados, DLQ y envíos tardíos; no acredita recuperar mensajes desde un backup de definiciones.
- [ ] Aprobar finalidad, aviso a usuarios, mínimos y retención por tipo de dato;
  aceptar política de privacidad instalada. USB/inventario/osquery tienen alcance separado.
- [ ] Completar aceptación de mantenimiento con concurrencia, datos tardíos, reintentos y
  columnstore/workers del destino. CI de retención aprobado no sustituye dimensionamiento real.
- [ ] Decidir archivo histórico adicional; implementarlo solo si se requiere, con cifrado,
  catálogo, recuperación y vencimiento.
- [ ] Configurar destino externo, programación, alertas y RPO/RTO; restaurar integralmente
  en entorno limpio, medir tiempos y reconciliar mensajes/datos antes de activar borrado.
- [ ] Aceptar cortes durante transferencia y migración con agente/DB/TLS reales.

## 4. Agente multiplataforma

- [x] Implementar supervisor de conexión 0.2.4: comprobar conexión/canal reales,
  reconstrucción serializada, límite de inicialización y reintentos acotados.
- [x] Cargo check 0.2.4 y regresión real de canal/conexión cerrados, reconexión
  concurrente única y publicación confirmada aprobados con broker temporal.
- [x] Compilar, empaquetar e instalar MSI 0.2.4 sin firma; configuración idéntica,
  servicio Running y sesión activa. Reconexión tras recuperación de RabbitMQ
  sin reiniciar agente; cola de más de 1.100 eventos a 1, datos recientes en API.
- [ ] Publicar el supervisor: conservar los bytes del MSI 0.2.4 entregado; si CI
  reconstruye el paquete, usar una versión posterior. No sobrescribir artefactos.

- [x] Drenaje acotado de caché implementado; ensayo de 5.000 eventos AMQPS → PostgreSQL
  aprobado sin duplicar efectos. IDs conservados y confirmación SQLite atómica.
- [x] Servicio/sesión separados, IPC autenticado y vista local loopback de lectura.
- [x] Icono verde/rojo basado en estado real de conexión/canal, consultado por IPC autenticado.
- [x] Compilación/ciclo de vida CI Windows, Ubuntu 22.04/24.04 y contenedores Linux aprobados.
- [ ] Aceptación nativa Windows/Ubuntu/Debian: reboot, parada intencional, shutdown,
  usuario limitado, sesiones múltiples, RDP/XDG, permisos IPC y captura real.
- [ ] Completar escenarios de disco/cola llena o corrupta, identidad dañada, escritura
  interrumpida y recuperación sin pérdida silenciosa sobre la entrega final.
- [x] Medición corta Windows 0.2.4: CPU, RAM, E/S de proceso, disco total, tráfico
  AMQP y cola; reenvío de 2.739 eventos y corte/recuperación comprobados.
  Ver [AGENT_PERFORMANCE.md](AGENT_PERFORMANCE.md); CPU baja, RAM pico 171,73 MiB.
- [ ] Identificar/optimizar auxiliares del pico de memoria (>150 MiB provisional),
  completar disco físico y red total por proceso si se exige atribución estricta.
- [ ] Carga sostenida, respuesta de aplicaciones, equipos de pocos recursos y Linux;
  validar aparición/desaparición de dispositivos/escritorio. Wayland no admite
  captura global implementada. No certificar ausencia de ralentización con ensayo corto.
- [ ] Fijar versiones SO soportadas y límites de la matriz antes de declarar soporte estable.

## 5. Instaladores y actualización

- [x] Generadores MSI/DEB, enrolamiento separado del paquete genérico y versiones coherentes.
- [x] Reparación Windows real conservando configuración y reanudando servicio/sesión (20/11).
- [x] Ciclo DEB de instalación/reinstalación/eliminación/purga en contenedores con systemd
  simulado; no sustituye servicio/escritorio nativos.
- [x] Publicación automática desde checkout del commit aprobado; hashes y notas incluidos.
- [x] Actualizador Windows oculto bajo SYSTEM, canales candidate/stable, verificación de
  origen/HTTPS/SHA256, política protegida y protección contra downgrade; pruebas focalizadas.
- [x] Corregir en 0.2.3 reintentos previos a instalación; mantener bloqueado el reintento
  automático de una instalación fallida de la misma versión para revisión de TI.
- [x] Actualización real Windows desde GitHub mediante tarea SYSTEM aceptada el 21/11;
  versión 0.2.3, configuración intacta, servicio conectado y sesión activa.
- [ ] Completar rollback, cortes, sesiones múltiples y conservación detallada de
  identidad/claves/outbox con artefactos finales; completar casos de
  journals/WAL/enlaces/purga/archivos ajenos y transición heredada soportada.
- [ ] Analizar reproducibilidad de paquetes finales y completar manifiestos de procedencia.
- [ ] Evolución posterior: actualización automática Linux/APT y distribución central a escala.

**Firma aplazada por decisión del usuario:** no bloquea el candidato sin firma autorizado.
Mantener hashes/canal confiable y aceptación explícita; no afirmar identidad del publicador
verificada. Firma Windows y confianza Linux siguen como mejora futura documentada en
[RELEASE_SIGNING.md](RELEASE_SIGNING.md). Serie vigente **0.2.x**; no reutilizar una versión
para bytes distintos. Procedimiento: [AUTOMATIC_UPDATES.md](AUTOMATIC_UPDATES.md).

## 6. Dashboard

- [x] Renovación UX, tema oscuro predeterminado, inicio con equipos/conexión y detalle
  por máquina; Wi-Fi/USB/inventario sin grupo lateral independiente.
- [x] Lint/build y regresión Chromium de XSS en nombres/títulos/búsquedas/USB/WiFi,
  con y sin CSP; bloqueo inline comprobado e incorporado al CI.
- [ ] Completar exportaciones, inspección de assets/almacenamiento sin secretos,
  cierre/revocación, auditoría y permisos por rol en el destino final.
- [ ] Aceptación humana: vacíos/atrasos/errores, temas, móvil, teclado y lector de pantalla.

## 7. Aceptación final y despliegue gradual

- [x] Publicar código y candidato 0.2.3 tras CI; publicar no equivale a desplegar producción.
- [x] Conciliar TODO con evidencias y separar implementación de aceptación (21/11).
- [x] Limpiar compilaciones/cachés/entregas antiguas y TODO duplicados (21/11);
  conservar configuración, herramientas y evidencias. Ver REPOSITORY_LAYOUT.md.
- [ ] Definir cantidad de equipos, duración, capacidad, responsables, criterios de éxito,
  versión/hashes y umbrales de parada/rollback antes del piloto final.
- [ ] Ensayo integral con artefactos definitivos: agente → RabbitMQ → DB → dashboard,
  ACK/duplicados/concurrencia/DLQ/eventos inválidos/tardíos, desconexiones y recuperación.
- [ ] Matriz final de seguridad y dependencias en destino; carga sostenida, crecimiento DB,
  latencia y recursos. Repetir solo lo afectado por cambios, no toda prueba histórica.
- [ ] Emitir informe único de defectos/evidencias/riesgos residuales y mantener README,
  CHANGELOG y runbooks sincronizados con cada entrega.
- [ ] Aprobar privacidad/retención/riesgos, verificar backup recuperable y soporte;
  autorizar producción y desplegar gradualmente con alertas comprobadas.

## Orden de ejecución restante

1. Publicar el supervisor y observar estabilidad tras la recuperación del puerto
   Docker. 0.2.4 instalado localmente; GitHub permanece en 0.2.3.
2. Resolver hallazgos de seguridad/imágenes y revocación; cerrar configuración DevOps.
3. Completar aceptación nativa por SO y respaldos externos/restauración.
4. Ejecutar piloto integrado acotado, emitir informe y autorizar despliegue gradual.

No asignar un porcentaje único: implementación, distribución y aceptación productiva
son estados diferentes. Kubernetes permanece como evolución posterior.
