# Entrega 0.2.1 — sin firma

Fecha: 2026-11-20. Candidato para despliegue controlado. La firma queda aplazada
por decisión del responsable; esto no certifica la aceptación de producción.
El piloto instalado sigue en 0.2.0 hasta que se ejecute su actualización.

## Contenido

- Windows x64: MSI y asistentes de instalación, reparación y desinstalación.
- Linux amd64: DEB separados para Debian 12, Ubuntu 22.04 y Ubuntu 24.04.
- Código completo del servidor, dashboard, agente y despliegue Docker, sin
  credenciales ni datos locales. El servidor y el dashboard se construyen con
  las instrucciones de [DevOps](RELEASE_DEVOPS.md).
- SHA256 y manifiestos de procedencia. Un hash detecta cambios, pero no sustituye
  una firma ni un canal de distribución confiable.

## Instalación Windows sin firma

Extraer el ZIP Windows completo. Comparar el SHA256 del MSI con SHA256SUMS
obtenido de la entrega aprobada por TI. Desde PowerShell en esa carpeta:

```powershell
.\setup.ps1 -Action install -PackagePath .\monitor-agent-0.2.1-windows-x64.msi `
  -LocalTest -ExpectedSha256 '<SHA256 publicado del MSI>'
```

El nombre `LocalTest` se conserva para exigir aceptación explícita de paquetes
sin firma. No desactiva las comprobaciones del canal firmado ni instala confianza
en certificados. Windows puede mostrar publicador desconocido. Para reparar o
desinstalar, usar `-Action repair` o `-Action remove` con el mismo paquete y hash.
La configuración y el enrolamiento se completan según la guía de DevOps; no se
incluyen credenciales del piloto. No ejecutar directamente el MSI omitiendo el
coordinador. La desinstalación conserva identidad/configuración/datos pendientes.

## Linux

Elegir el paquete correspondiente a la distribución, verificar SHA256 y ejecutar
`sudo apt install ./monitor-agent-0.2.1-amd64.deb`. Configurar
`/etc/activity-monitor/.env` y enrolar el equipo antes de iniciar el servicio,
según la guía de DevOps. No distribuir el archivo de configuración con secretos.
Los paquetes son amd64; no se afirma compatibilidad ARM64 ni captura global Wayland.

## Pendiente para producción

Aceptar los artefactos finales en equipos reales: actualización, reinicio,
sesión de escritorio, desinstalación y reversión. Configurar credenciales/TLS del
destino y SMTP autorizado; el correo con contraseña habitual no fue entregado.
Completar la revisión de hallazgos de imágenes y la migración del piloto.
Firma organizacional y repositorio APT firmado quedan para una siguiente entrega.
Consultar [TODO](TODO.md) y [certificados internos](RELEASE_SIGNING.md).
