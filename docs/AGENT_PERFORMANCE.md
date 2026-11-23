# Impacto medido del agente — 2026-11-22

## Resultado y alcance

Agente **0.2.4** instalado en Windows, dos procesos (servicio y escritorio),
**16 procesadores lógicos**. Se observaron aproximadamente cuatro minutos de uso
real, incluyendo una cola acumulada, un corte controlado del broker local de 30 s
y 90 s de observación después de arrancarlo. El primer tramo de 120 s incluye
reenvío y uso estable. No es una comparación A/B de respuesta de aplicaciones.

La CPU observada es baja. **No se certifica impacto cero ni ausencia de ralentización
en todos los equipos.** La memoria combinada superó la referencia provisional de
150 MiB en una muestra con dos procesos auxiliares; requiere clasificación/optimización.

| Tramo | CPU promedio total | CPU p95 | CPU máximo | RAM residente promedio | RAM máxima |
|---|---:|---:|---:|---:|---:|
| Reenvío de cola acumulada | 0.62% | 0.87% | 0.87% | 108.1 MiB | 108.2 MiB |
| Uso estable posterior | 0.08% | 0.24% | 0.45% | 83.8 MiB | 171.7 MiB |
| Broker detenido | 0.10% | 0.67% | 0.67% | 69.6 MiB | 69.8 MiB |
| Recuperación del corte | 0.09% | 0.28% | 0.34% | 74.5 MiB | 81.6 MiB |

CPU total se normaliza por los 16 procesadores lógicos: el 0,62% durante reenvío
corresponde aproximadamente al 9,95% de un núcleo, no al 0,62% de un núcleo.
p95 es el percentil 95 de las muestras; máximo es un intervalo observado, no un
máximo instantáneo. RAM residente suma working sets y puede contar páginas compartidas
más de una vez. El pico de memoria privada comprometida fue 159,28 MiB.

## E/S y tráfico

| Tramo | Lectura media del proceso | Escritura media del proceso |
|---|---:|---:|
| Reenvío acumulado | 466.5 KiB/s | 410.2 KiB/s |
| Uso estable | 36.8 KiB/s | 45.9 KiB/s |
| Desconexión | 0.9 KiB/s | 6.9 KiB/s |
| Recuperación | 0.0 KiB/s | 7.9 KiB/s |

Los contadores de E/S de Windows **incluyen archivos, dispositivos y red**; no son
bytes físicos de disco atribuibles exclusivamente al agente. Durante el reenvío
hubo un pico muestreado de 2,07 MiB/s de lectura y 1,96 MiB/s de escritura de proceso.
Se midió también el disco total del anfitrión, pero allí conviven Docker y otras
aplicaciones: no atribuir ese tráfico al agente. Falta una traza física por proceso
si se requiere aislar completamente la presión de disco.

RabbitMQ contabilizó aproximadamente **5,30 MiB enviados por el agente y 62 KiB
recibidos** durante el tramo inicial de 120 s; incluye el reenvío acumulado. En la
recuperación breve: aproximadamente 8,4 KiB enviados y 0,7 KiB recibidos. Son
contadores AMQP de la única conexión desde el gateway Docker, no tráfico de tarjeta
física ni consumo de Internet. Los contadores del broker pueden actualizarse con
retraso: no atribuir con precisión cada byte al límite entre reenvío y uso estable.
Durante la detención, la medición del broker no está disponible; no registrar cero
como tráfico demostrado. No incluye tráfico HTTP del agente ni intentos TCP fallidos.

## Recuperación y experiencia observada

- Antes de arrancar Docker: 2.966 eventos pendientes. Al comenzar la captura: 2.739.
  En unos 28 s de observación bajaron a 2, manteniendo captura activa.
- Después del corte de 30 s: conexión observada unos 30 s después de la primera
  muestra de recuperación; cola de hasta 11 eventos y descenso a 2 hacia los 75 s.
- Cierre: versión 0.2.4, conectado, 1 evento pendiente; RabbitMQ running/healthy,
  sin pausa. No se reinició el agente para recuperar la conexión.
- Servicio y sesión sin ventana principal visible en la comprobación final. Esto
  no prueba por sí solo ausencia de todos los avisos durante toda la sesión.

Los tiempos incluyen detección/reintento y arranque del broker local. No son un SLA.
No se midieron latencia de teclado, fluidez de aplicaciones o un equipo de pocos
recursos. La CPU del anfitrión alcanzó 85% en el tramo inicial, con otros procesos
activos: no confundirla con la CPU del agente.

## Método y límites

- Muestreo objetivo cada 2 s; deltas de CPU kernel/usuario, E/S acumulada, memoria,
  salud/cola local, contadores de broker, CPU y disco totales del anfitrión.
- Incluye descendientes visibles. Procesos auxiliares que nacen y terminan entre
  muestras pueden omitirse; su primer intervalo de CPU no se reconstruye. El pico
  de 171,73 MiB coincidió con dos auxiliares, sin nombres registrados en esta captura;
  no atribuirlo a un recolector concreto sin evidencia adicional.
- El observador tiene coste propio y comparte anfitrión con Docker. No es ensayo
  prolongado ni aceptación Windows/Linux completa.
- Referencias provisionales previas al informe: CPU media <1% total, p95 <3% y
  working set máximo <150 MiB. CPU cumple; **memoria no cumple en una muestra**.
- Primera captura descartada para CPU: `Math.Max(0, fracción)` elegía la sobrecarga
  entera y redondeaba. Corregido con `0.0`, contrastado con contadores acumulados
  Windows/Get-Process. No se reutilizan los ceros iniciales como resultados válidos.
- El ensayo preliminar con pausa del contenedor reprodujo el cierre del puerto Docker
  después de reanudar. Se recuperó recreando solo RabbitMQ con el volumen conservado.
  La medición válida usa parada/arranque; queda observar la recurrencia del puerto.

## Reproducción y evidencia

Desde la raíz, con Docker y el piloto disponibles (interrumpe solo el broker local):

```powershell
.\scripts\measure-agent-impact.ps1 -NormalSeconds 120 -OfflineSeconds 30 `
  -RecoverySeconds 90 -InterruptLocalBroker `
  -OutputDirectory .local-test/performance-nueva-medicion
python scripts/summarize-agent-impact.py .local-test/performance-nueva-medicion
```

Sin `-InterruptLocalBroker`, no detiene el broker. La carpeta debe ser nueva.
El script conserva datos y arranca el broker en finally si la medición falla.
No captura títulos, pulsaciones, mensajes ni credenciales en los resultados.

Evidencia privada: `.local-test/performance-0.2.4-corrected-20261122/`, archivos
`samples.jsonl`, `metadata.json` y `summary.json`. No publicar `.local-test`.

## Pendientes

- Identificar los auxiliares del pico de RAM y reducir o aceptar su presupuesto
  con criterio de equipos objetivo; no cambiar el umbral solo para darlo por aprobado.
- Medir sesiones de trabajo prolongadas, inventario/USB/osquery y equipos con menos
  recursos, incluyendo Linux; comparar respuesta de aplicaciones con/sin agente.
- Si se requiere una garantía de disco físico/red total por proceso, ampliar la
  instrumentación: los contadores actuales no bastan para esa afirmación.
- Observar estabilidad del puerto Docker y publicar el supervisor según TODO.
