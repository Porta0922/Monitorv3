# Reenvío y deduplicación

El servidor acepta telemetría con timestamp RFC3339 de hasta 30 días de antigüedad y 120 segundos de adelanto. El límite se aplica antes de cualquier escritura, también al reenvío administrativo. Los mensajes fuera del intervalo o sin fecha válida van a la cola .dead sin reintentos automáticos; no se eliminan silenciosamente.

processed_events conserva las marcas al menos 32 días desde recepción. La limpieza comparte el mantenimiento por lotes y permanece desactivada con HISTORY_RETENTION_ENABLED=false. DEDUPE_RETENTION_DAYS puede aumentarse; valores de 1 a 31 son rechazados y 0 desactiva la tarea.

La ventana de aceptación es fija deliberadamente: ampliarla después de limpiar marcas permitiría reproducir efectos ya confirmados. Su ampliación requiere una migración de política y un período de transición. El reenvío debe preservar event_id, timestamp y cuerpo. No se debe corregir manualmente la fecha de un evento antiguo para ingresarlo como nuevo.

Los filtros específicos de cada recolector pueden ser más estrictos: la ventana global no obliga a almacenar muestras que ya no sean válidas para su agregación. Agentes desconectados más de 30 días conservan mensajes hasta confirmación del broker; el servidor los conserva en DLQ para tratamiento administrativo. No hay purga automática de DLQ.

Implementado en código; despliegue y validación pendientes en P4. Antes de habilitar limpieza deben retirarse todos los consumidores antiguos que no apliquen este límite.
