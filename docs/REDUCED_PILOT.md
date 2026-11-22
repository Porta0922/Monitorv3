# Piloto reducido con DevOps — preparación para 2026-11-23

Estado verificado el 22/11: GitHub/main y release pública en **0.2.3**;
el agente local está en **0.2.4**. Este cambio prepara la publicación **0.2.5** con supervisor, limpieza, TODO y
mediciones. Antes de instalar, comprobar que CI y Build and Release terminaron
y que existe `agent-v0.2.5` con sus paquetes; producción no está aprobada.

## Condición de entrada

Publicar código y documentación mediante rama/PR, exigir CI satisfactorio y fijar
SHA, versión y hashes de los paquetes que se usarán. El MSI 0.2.4 ya fue entregado
localmente: si CI lo reconstruye, usar una versión posterior (0.2.5), sin sobrescribir
los bytes de la entrega anterior. No mezclar servidor de un commit con un agente
anterior esperando que incluya el supervisor nuevo.

Si la publicación no está lista, DevOps puede preparar infraestructura y secretos,
pero no registrar como aceptado un piloto de una versión todavía inexistente.

## Alcance propuesto

- Empezar con 3–5 equipos Windows autorizados y representativos, incluyendo el de
  menos recursos; observar 24–48 horas de trabajo real antes de ampliar.
- Servidor Linux dedicado de ensayo con Docker Engine/Compose, separado de producción.
  El fallo local observado pertenece al puerto de Docker Desktop; no se presume
  solucionado por cambiar de anfitrión, se comprueba el nuevo destino.
- Linux como agente queda para una aceptación nativa separada; los DEB construidos
  no certifican aún captura/escritorio en todas las distribuciones.
- Instaladores sin firma: aceptación administrativa explícita y SHA256 verificado.
  No presentar esta excepción como firma válida ni estabilidad final.

## Orden para la sesión con DevOps

1. Revisar release/CI/hashes y anotar responsables, equipos, fecha y criterio de parada.
2. Aplicar [RELEASE_DEVOPS.md](RELEASE_DEVOPS.md): credenciales propias, cuentas/MFA,
   HTTPS/AMQPS, redes restringidas y secretos por archivo. No copiar el .env del piloto
   personal a GitHub ni al entorno productivo. Confirmar revocación del token histórico.
3. Definir política de captura/privacidad, retención, destino de respaldo y responsables
   de alertas. El correo real sigue pendiente de un relay o contraseña de aplicación.
4. Arrancar DB, RabbitMQ, servidor, dashboard y mantenimiento; verificar salud y puertos.
   Redis no forma parte del despliegue vigente.
5. Instalar/enrolar primero un equipo, verificar identidad y datos recientes en dashboard;
   después añadir los restantes. Conservar identidad/configuración/cola al actualizar.
6. Observar uso normal, bloqueo/desbloqueo, reinicio, desconexión y reenvío; confirmar
   ausencia de ventanas inesperadas y fechas de datos, no solamente un icono verde.
7. Medir CPU/RAM/E/S/red y experiencia de trabajo en los equipos objetivo. Usar
   [AGENT_PERFORMANCE.md](AGENT_PERFORMANCE.md) como referencia, no como garantía.
   El medidor incluido está acoplado al piloto local Windows: no ejecutarlo sin
   adaptación contra un servidor remoto; su opción de interrupción afecta al broker local.
8. Restaurar un respaldo en entorno aislado, revisar logs/DLQ y ensayar reversión;
   documentar hallazgos y aceptar o detener la ampliación.

## Criterios de salida

- Datos recientes y correctos por equipo; recuperación sin pérdida evidente ni
  crecimiento permanente de la cola; no confundir ACK broker con persistencia final.
- Sin ventanas molestas ni deterioro perceptible del trabajo habitual, respaldado
  por medición; revisar el pico de RAM de **171,73 MiB** con auxiliares observado aquí.
- Configuración/identidad conservadas durante actualización/reinicio; procedimiento
  de retirada y rollback disponible.
- Hallazgos, riesgos residuales, soporte y próximos pasos registrados. Credenciales,
  TLS, backup y alerta no aceptados se mantienen explícitamente pendientes.

Ante desconexión persistente, crecimiento sostenido de cola/CPU/RAM o molestia al
usuario: detener incorporación de equipos y diagnosticar; no purgar caché para
ocultar el problema. Un piloto reducido satisfactorio no autoriza automáticamente
el despliegue general.
