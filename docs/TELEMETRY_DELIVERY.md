# Recepción transaccional y recuperación de telemetría

Implementación: 2026-10-26. No equivale a entrega exactamente una vez en el transporte.

## Frontera de confirmación

El consumidor usa TelemetryWriter: una transacción PostgreSQL engloba el marcador
processed_events, registro del dispositivo y todas las escrituras del evento.
La restricción única del event_id serializa reenvíos concurrentes. Un error revierte
la transacción completa; ya no se ignoran errores de persistencia de heartbeat o seguridad.
Los endpoints HTTP reutilizan las mismas escrituras mediante transacciones propias.

Solo después de commit se emite ACK al broker. Si se pierde ese ACK, el reenvío encuentra
el marcador y no repite los efectos. Límites por transacción: lock_timeout 5 s y
statement_timeout 30 s. No se eliminan marcadores en este cambio.

## Fallos y mensajes inválidos

Cada cola original conserva sus argumentos y recibe una cola durable hermana .dead.
No se purgan ni recrean colas existentes. Prefetch: 16 por consumidor.

Un fallo de persistencia admite tres republicaciones, separadas por una espera de
2 s dentro del consumidor. La copia conserva el cuerpo original y propiedades, añade
x-monitor-attempts y fuerza delivery_mode=2. Se publica a la cola original mediante
el exchange por defecto, con mandatory y publisher confirms. Solo tras Ack sin retorno
se confirma el original. Si falla esa transferencia o el ACK, se cierra la conexión y
el broker conserva/reencola los originales no confirmados.

Tras agotar reintentos se transfiere del mismo modo a <cola>.dead. JSON inválido o
identidad de dispositivo inválida va directamente allí. No hay TTL ni borrado automático
en .dead. Debe alertarse y revisarse su tamaño; automatización de alertas/replay pendiente.
La secuencia publicar-confirmar-ACK puede producir copias si se corta la conexión entre
sus pasos: los efectos SQL se deduplican, pero la DLQ puede contener copias repetidas.

Reprocesar después de resolver la causa, conservando event_id y cuerpo. No borrar
processed_events para forzar replay. Las marcas históricas creadas por versiones antiguas
no permiten reconstruir eventos ya confirmados y perdidos; este cambio no los recupera.

## Evidencia

- PostgreSQL/TimescaleDB temporal: fallo inyectado en recursos revierte dispositivo,
  actividad y marca; 16 llamadas concurrentes con el mismo ID producen una sola muestra.
- RabbitMQ en vhost temporal: fallo SQL llega a .dead después de 3 reintentos, ningún
  efecto parcial, replay reparado con 8 copias produce una muestra; JSON inválido conservado.
- Pruebas ignoradas por defecto para no contactar infraestructura accidentalmente.
  Se ejecutan con TEST_DATABASE_URL, TEST_RABBITMQ_MANAGEMENT_URL y
  TEST_RABBITMQ_BASE_URL contra infraestructura desechable. CI configurada; no ejecutada remotamente.

Pendientes: prueba de corte exacto entre confirmación y ACK, observabilidad/replay de DLQ,
ventana máxima de reenvío y limpieza compatible de processed_events.

## Drenaje del agente tras desconexiones — 2026-11-07

El servicio reenvía hasta 256 pendientes por lote, con orden de inserción estable y un presupuesto de publicación de cinco segundos. Si completa un lote lleno o agota el presupuesto después de confirmar algún evento, espera 250 ms antes del siguiente; sin pendientes, sin conexión, sin progreso o tras un error de publicación espera veinte segundos. No cambia IDs, timestamps ni cuerpos para hacer pasar mensajes antiguos como recientes. Los eventos fuera del horizonte permitido siguen sujetos a la política de replay del servidor.

Solo se marca como sincronizado lo que recibió confirmación del publicador. Un timeout puede ocurrir después de que el broker haya recibido el evento: el agente conserva su ID y permite un reenvío deduplicable. Las confirmaciones locales se guardan juntas en una transacción SQLite; si falla alguna escritura se conserva todo el lote. La limpieza de confirmados se ejecuta cada cinco minutos, conservando la ventana de tres días existente.

El presupuesto limita la espera de red por pasada; no es un límite estricto para el acceso SQLite o la planificación del sistema operativo. Esta implementación corresponde al drenaje servicio → broker; la sesión → servicio mantiene su bucle independiente. El estado actual del servidor compara timestamps observados antes de reemplazar recursos/salud; el inventario conserva su control de snapshots atrasados.

Regresión focalizada: 5000 eventos cifrados, lotes acotados, mismos IDs y orden, corte parcial, timeout y rollback de confirmaciones por error de escritura. Además, recorrido aislado por AMQPS aprobado con 5000 eventos: 5000 muestras y marcadores en PostgreSQL, sin duplicar efectos y con cola del broker vacía. Los reenvíos de confirmación incierta produjeron copias de transporte que fueron deduplicadas. Evidencia privada en `.local-test/security/outbox-backlog.log`. No equivale a carga sostenida de una oficina ni a aceptación del agente instalado en cada sistema operativo.
