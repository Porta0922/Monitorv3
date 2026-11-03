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
