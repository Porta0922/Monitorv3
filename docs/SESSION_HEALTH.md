> Actualización 2026-10-28: el cierre de implementación, los nuevos flujos multiusuario
> y la integración de paquetes están descritos en [P2_IMPLEMENTATION_CLOSURE.md](P2_IMPLEMENTATION_CLOSURE.md).
> Las limitaciones y pendientes de esta página anteriores a esa fecha son históricos;
> la validación funcional continúa en P4.

# Diagnóstico consolidado de sesiones

Implementado el 2026-10-28; sin compilar, probar ni desplegar. Forma parte de P2.

## Flujo

La sesión supervisa una tarea session_health independiente de la política de captura.
Cada 30 segundos envía un diagnóstico por el IPC AES-GCM existente. Incluye UUID de
instancia del proceso, secuencia, disponibilidad de entrada, pausa, cola pendiente,
vida de wrappers supervisados y huella SHA256 de la política efectiva. No transmite
usuario, títulos de ventanas, claves ni métricas CPU/RAM.
El servicio exige identidad del equipo, UUID válido, secuencia creciente por instancia,
fecha a no más de 30 segundos del reloj local, tipos y límites del contrato. La secuencia
no se reinicia al reiniciarse la tarea supervisada dentro del mismo proceso.
Se aceptan hasta 16 instancias observadas; se retiran tras 15 minutos sin diagnóstico.
Las observaciones se mantienen en memoria, se pierden al reiniciar el servicio y se
incluyen en su salud cada 300 segundos. El servidor conserva únicamente el estado más
reciente del equipo; no agrega una tabla de historial ni duplica telemetría de recursos.

El diagnóstico es directo: no se encola offline. Reintenta en el siguiente intervalo;
no convertir un diagnóstico antiguo en señal actual por reenviarlo desde la outbox.
La actividad continúa usando su entrega durable y ACK después de SQLite, sin cambios.
En el servicio una observación se considera atrasada tras 90 segundos. El dashboard
además considera la antigüedad del reporte consolidado y 390 segundos desde recepción
(300 de publicación más 90 de tolerancia). Las fechas se muestran explícitamente.
Esto ofrece diagnóstico periódico, no presencia en tiempo real.

## Límites y significado

La autenticación corresponde al secreto enrolado existente, no a una identidad de
usuario comprobada por el SO. UUID identifica un proceso, no un usuario ni sesión RDP.
Varias filas pueden resultar de reinicios. No acreditan soporte de actividad concurrente.
Se mantiene la concesión exclusiva de 90 segundos para entregar actividad: los reportes
no la adquieren ni renuevan. No se altera la semántica de totales históricos por equipo.
Pausa, vida del wrapper, disponibilidad de entrada y coincidencia de política son
señales distintas. Una tarea supervisada puede estar esperando disponibilidad.
Ausencia de reportes no demuestra ausencia de usuarios ni identifica una causa concreta.

El wrapper de sesión ahora observa cambios de elegibilidad cada segundo aun sin una
nueva política: activa captura al aparecer la entrada nativa y detiene el trabajador
si deja de ser elegible. El diagnóstico de ventana distingue observación de ausencia
inconclusa; no inventa un error nativo cuando el adaptador solo devuelve None.

## Trabajo restante

Multiusuario/RDP requiere credenciales independientes por principal, rutas y exclusión
por sesión, enrolamiento/revocación múltiple, origen persistido por evento y contratos
de agregación que no sumen tiempo concurrente como tiempo del equipo. No basta quitar
la concesión actual. Este bloque sigue pendiente de implementación P2.
Errores nativos detallados por adaptador también permanecen abiertos.

P4: compatibilidad con agentes antiguos, límites y replay del diagnóstico, caída del
servicio, pausa/offline, reinicio de tarea, relojes, aparición tardía de entrada,
conservación de outbox, renderizado de estado antiguo y comparación de políticas.
