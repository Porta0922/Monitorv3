# Monitor Agent 0.2.2 — candidato sin firma

Esta entrega incorpora actualización silenciosa de Windows y el estado de
conexión del servicio en el icono de la sesión. No constituye todavía la
aceptación final de producción.

## Instalación inicial

Descargar el ZIP `monitor-agent-VERSION-windows-x64.zip`, extraerlo y ejecutar
`Instalar-sin-firma.cmd` como administrador. Esta primera instalación necesita
privilegios: los agentes 0.2.0/0.2.1 no tienen un actualizador activo que pueda
instalarlo por sí solo. El servidor/enrolamiento se configuran por separado.
No se incluyen credenciales. Se conservan configuración, identidad y cola local.

## Actualizaciones posteriores de Windows

La instalación registra `ActivityMonitor-Updates` como SYSTEM, sin sesión
interactiva. Comprueba GitHub cada seis horas, con retraso aleatorio de hasta
30 minutos para distribuir las consultas. PowerShell y MSI se ejecutan ocultos,
con instalación `/qn /norestart`: sin CMD, mensajes al usuario ni reinicio
forzado. El coordinador detiene los procesos del agente durante la sustitución
y reanuda las sesiones enroladas; el actualizador inicia el servicio.

Solo considera etiquetas `agent-vX.Y.Z` y el MSI Windows x64 de esa versión;
ignora las releases heredadas 3.x, borradores y versiones anteriores/iguales.
El canal `candidate` permite candidatos 0.x; `stable` excluye prereleases.
Descarga exclusivamente por HTTPS desde GitHub y sus hosts de assets, con
límites de tamaño y comprobación contra el SHA256 proporcionado por GitHub.
Esto confía en la cuenta/repositorio y en TLS: no equivale a firma de publicador.
La instalación sin firma habilita explícitamente esa política, según la decisión
del responsable. Una política firmada requiere el publicador aprovisionado.

TI administra `C:\ProgramData\ActivityMonitor\Updates\policy.json`:

```json
{"schema":1,"enabled":true,"allow_unsigned":true,"channel":"candidate"}
```

Solo administradores/SYSTEM pueden modificarla. Cambiar `enabled` a `false`
pausa nuevas actualizaciones. `status.json` registra versión, resultado y hora,
sin secretos. Un intento fallido/interrumpido de la misma versión requiere
revisión de TI antes de retirar ese bloqueo; no entra en un bucle de reinstalación.
Los errores HTTP/límites de GitHub se revisan en la siguiente ejecución.
En redes con muchos equipos detrás de una sola IP, la cuota pública de GitHub
puede demorar las consultas; planificar un distribuidor central antes de ampliar
el despliegue. El actualizador no almacena tokens de GitHub.

Desinstalar con el coordinador elimina la tarea de actualizaciones. La carpeta
de estado se conserva para diagnóstico, junto con identidad y datos pendientes.

## Icono de conexión

Verde: el servicio tiene conexión y canal RabbitMQ activos. Rojo: broker
desconectado, servicio detenido o comunicación autenticada con el servicio
no disponible. Se consulta cada cinco segundos (con timeout); el tooltip
explica el estado. Verde no certifica que el servidor ya guardó cada evento
en la base de datos. No se muestran globos ni avisos automáticos.

## Publicación

Cada cambio distribuible debe incrementar `agent/Cargo.toml` y `Cargo.lock`.
Tras fusionar en `main` y pasar las cuatro comprobaciones requeridas, el flujo
Build and Release compila MSI/ZIP Windows, DEB Debian12/Ubuntu22.04/Ubuntu24.04,
código fuente y SHA256SUMS; publica `agent-vVERSION` como candidato.
No sobrescribe versiones ya publicadas. v3.4.0 se conserva como legado y deja
de anunciarse como release estable. No usar `/releases/latest` para este canal.

Los DEB se publican para instalación administrativa; el mecanismo silencioso
de esta entrega es para Windows. La distribución automática Linux mediante
APT firmado sigue pendiente, al igual que la firma organizacional de Windows.
Ver [firma interna](RELEASE_SIGNING.md), [DevOps](RELEASE_DEVOPS.md) y [TODO](TODO.md).
