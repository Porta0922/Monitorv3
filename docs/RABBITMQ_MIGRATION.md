# RabbitMQ: candidato y migración pendiente

Actualizado: 2026-11-17. No ejecutar una sustitución de imagen sobre el volumen 3.12 existente. El piloto conserva su broker actual.

## Ensayo de transferencia y reversión aprobado

`python tests/broker_cutover.py` crea dos brokers desechables y una red propia;
no accede al piloto. El ensayo local pasó con las imágenes 3.12 y 4.2 fijadas por
digest: 70 mensajes persistentes transferidos, incluidos duplicados y una cola
`.dead`; dos eventos adicionales enviados después del cambio; reinicio del
destino; devolución de los 72 mensajes al origen. Compara hashes de todos los
payloads y su multiplicidad, y exige cero entregas sin confirmar en el origen.
La evidencia sin credenciales queda en `.local-test/broker-cutover/`. Los
contenedores, volúmenes anónimos y red de ensayo se eliminan al terminar.
Se incorpora como job independiente del workflow de stack; ese CI aún debe
ejecutarse sobre el commit publicado.

La transferencia usa Shovel con `ack-mode: on-confirm` y
`src-delete-after: queue-length`, para una ventana de drenaje con productores
pausados. No usar esa condición como migración continua: solo cubre la cantidad
existente al iniciar la transferencia. No se purgan colas ni se reutilizan
volúmenes entre versiones. Referencia: [configuración oficial de Shovel](https://www.rabbitmq.com/docs/4.2/shovel-dynamic).

Límites: los dos eventos tardíos simulan un productor que vuelve; no ejecutan
el outbox del agente real. El ensayo usa AMQP en red dedicada y administración
HTTP solo en loopback, con credenciales efímeras. No certifica TLS de Shovel,
deduplicación SQL, productores concurrentes, corte en medio de una confirmación,
carga sostenida ni una migración productiva. TLS del agente y deduplicación
de aplicación tienen ensayos separados; deben combinarse en la aceptación final.

Para el cambio real, mantener los consumidores y productores detenidos durante
el inventario y el drenaje; conservar origen, destino y outboxes hasta conciliar
IDs con los registros procesados. Las DLQ se trasladan sin reinterpretar ni
eliminar sus eventos, y solo se reprocesan tras corregir la causa y autorizarlo.
Si hay nuevas publicaciones antes de revertir, drenar también el destino y
conciliar sus IDs. DevOps debe validar los certificados y permisos del usuario
temporal de transferencia, y eliminarlo al concluir la ventana.

## Evidencia del candidato

Imagen RabbitMQ 4.2 management Alpine: `rabbitmq@sha256:77ca2ba3621a95208918e6e331199a6c81129690bf7fc570dc61b6e64726840a`.

Escaneo local sin avisos conocidos y recorrido aislado completo aprobado: sesiones/MFA/CSRF/roles, enrolamiento, publicación nativa AMQPS, renovación, recuperación, revocación y rechazo de CA no confiable. Esto acredita compatibilidad acotada, no migración de colas ni carga sostenida.

Para reproducir con los binarios debug del workspace construidos y las dependencias del ensayo instaladas:

```powershell
$env:TEST_RABBITMQ_IMAGE='rabbitmq@sha256:77ca2ba3621a95208918e6e331199a6c81129690bf7fc570dc61b6e64726840a'
try { python tests/identity-stack.py } finally { Remove-Item Env:TEST_RABBITMQ_IMAGE }
```

## Ruta propuesta para Monitor

Usar un broker nuevo con volumen separado. La estrategia de entornos separados permite migrar entre series sin reutilizar directamente los datos internos anteriores; importar definiciones no transfiere mensajes. Referencia: [guía oficial de migración blue-green](https://www.rabbitmq.com/docs/4.2/blue-green-upgrade).

1. Inventariar colas, DLQ, mensajes pendientes/sin confirmar, usuarios, permisos, certificados y políticas. Guardar definiciones y respaldos privados; contienen información sensible.
2. Preparar el broker candidato en red y volumen separados. Importar definiciones revisadas y configurar TLS, permisos individuales y límites equivalentes. No cambiar todavía los endpoints del piloto.
3. Ensayar transferencia con eventos sintéticos identificables: publicaciones concurrentes, desconexiones, duplicados y DLQ. Elegir y validar federación o una ventana de pausa con drenaje; no asumir que todos los agentes estarán conectados durante el cambio.
4. Coordinar enrolamiento/renovación y publicación con el cambio de destino. Conservar credenciales válidas y reconexión de agentes retrasados. Verificar ACK tras persistencia y deduplicación en DB.
5. Cambiar consumidores y productores según el mecanismo ensayado, drenar el origen y reconciliar eventos. Registrar contadores por cola, incluidos mensajes sin confirmar y DLQ; una cola vacía aislada no prueba ausencia de pérdida.
6. Mantener ambos entornos durante la observación. Para revertir después de nuevas publicaciones, reconciliar también los mensajes del destino antes de retirarlo; cambiar solo el endpoint puede abandonar datos.
7. Retirar el origen únicamente después de verificar drenaje, agentes reconectados y recuperación. Registrar digest, fecha, evidencia y responsable.

## Pendientes antes de aplicar

- [ ] Ensayo de transferencia y reversión sin pérdida silenciosa, con eventos duplicados y agentes desconectados.
- [ ] Política para DLQ y mensajes que no pueden procesarse.
- [ ] Capacidad, alertas, reinicio y recuperación del broker destino.
- [ ] Ventana de cambio y actualización reproducible de configuración.

Una instalación nueva puede usar un volumen vacío después de completar la aceptación del candidato. No confundir esa instalación con una migración que conserva datos. Si se elige actualización sobre el mismo volumen, revisar primero la [matriz oficial de versiones y feature flags](https://www.rabbitmq.com/docs/3.13/upgrade) para cada salto.
