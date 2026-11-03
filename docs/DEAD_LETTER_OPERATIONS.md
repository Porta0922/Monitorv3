# Operación de mensajes fallidos

El panel de mantenimiento consulta /maintenance/dead-letters cada minuto. Muestra mensajes listos por cola y avisa cuando alguna contiene pendientes. Una cola inaccesible se muestra como desconocida, no vacía. No es una notificación externa ni un conteo de entregas en curso.

POST /maintenance/dead-letters/:queue/replay exige autenticación administrativa, reason y limit entre 1 y 20. Solo admite las siete colas de telemetría conocidas. Guarda una intención de auditoría con actor y operación antes de acceder al broker; guarda el resultado al terminar. Si falta el registro final, el resultado puede ser parcial. No hay transacción distribuida entre PostgreSQL y RabbitMQ.

El reenvío usa basic_get con ACK manual y publica el mismo cuerpo y event_id en la cola original. Reinicia el contador de intentos. Confirma publicación obligatoria antes de reconocer el original. Un corte puede producir duplicados: la deduplicación transaccional sigue siendo necesaria. Un bloqueo de PostgreSQL evita dos reenvíos administrativos simultáneos; no impide que el consumidor normal procese mensajes publicados.

Se detiene ante una cabecera de cola con identidad/fecha inválida o fuera de la ventana. El mensaje vuelve a DLQ. No se sobrescribe su timestamp ni se purga la cola. Esos casos requieren un procedimiento de archivo/exportación específico; no deben repetirse sin resolver su causa. La UI informa la detención sin exponer el cuerpo de telemetría.

Tiempos acotados de conexión y operación. Ante error/timeout puede haber publicaciones confirmadas y un último resultado incierto: consultar conteos y auditoría antes de repetir. El conteo returned acknowledged representa originales reconocidos, no ingestión confirmada por el servidor.

Código implementado sin ejecución en esta fase. Validación RabbitMQ/DB/interfaz y cortes reservada a P4.
