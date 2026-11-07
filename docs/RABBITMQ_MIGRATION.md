# RabbitMQ: candidato y migración pendiente

Actualizado: 2026-11-07. No ejecutar una sustitución de imagen sobre el volumen 3.12 existente. El piloto conserva su broker actual.

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
