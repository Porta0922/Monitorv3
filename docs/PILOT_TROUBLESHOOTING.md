# Diagnóstico del piloto local

## Ventanas reportadas antiguas después de apagar Docker

El agente tiene dos componentes: el servicio y el proceso de la sesión interactiva. El servicio puede reportar conectividad mientras las instantáneas de ventanas todavía están atrasadas. Que `running_apps` figure como `unsupported_context` en el servicio es esperado: la captura de escritorio se comprueba en el diagnóstico de la sesión.

El 2026-11-04 se encontró Docker Desktop sin motor Linux disponible. El servicio y la sesión del agente seguían ejecutándose; la cola persistente del servicio acumuló eventos. Tras iniciar Docker, el diagnóstico confirmó `capture_ok` para actividad y ventanas y la cola comenzó a disminuir. La API conservaba instantáneas antiguas mientras recibía los eventos anteriores.

Pasos de diagnóstico:

1. Iniciar Docker Desktop y comprobar el piloto desde la raíz con `scripts/local-pilot.ps1 -Action status`.
2. En la computadora supervisada, abrir `http://127.0.0.1:9876` y revisar canal del publicador y eventos pendientes. Canal disponible no significa cola vacía.
3. En el dashboard revisar fechas de ventanas y Diagnóstico → sesiones: estado de captura y última observación.
4. Si los pendientes disminuyen, permitir el reenvío. En 0.1.0 el sincronizador procesa hasta 256 eventos por lote cada 20 segundos; el tiempo real depende de confirmaciones, errores y eventos nuevos.
5. Cuando las fechas sean recientes, actualizar Aplicaciones → Ventanas reportadas. No confundir la fecha de recepción de un heartbeat con la fecha de la instantánea de ventanas.

No borrar la caché ni reenrolar el agente para acelerar el drenaje: contienen eventos e identidad que deben conservarse. Si la cola no baja con el servidor disponible, revisar transporte y errores; si la sesión no produce eventos, revisar su tarea y permisos de escritorio.

El piloto depende de Docker Desktop en esta computadora. La persistencia del agente no inicia automáticamente la infraestructura del servidor. En producción, el servidor debe mantenerse disponible con la supervisión correspondiente.

## Resultado observado — 2026-11-04

Tras iniciar Docker Desktop, la cola del servicio descendió de 5.439 eventos (primer diagnóstico disponible) a cero. A las 17:03:37 de Paraguay (20:03:37 UTC), la API devolvió 12 aplicaciones con reportes recientes; la última instantánea era de las 17:03:05. Se recuperó sin reinstalar, reiniciar ni purgar el agente. Evidencia privada de recuentos/fechas: `.local-test/logs/window-recovery.json`. El drenaje observado tomó aproximadamente nueve minutos desde la primera consulta al servidor recuperado.
