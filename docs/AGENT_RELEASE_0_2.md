# Preparación del agente 0.2.0

## Aceptación focalizada — 2026-11-20

Reparación real del MSI piloto 0.2.0 aprobada usando el coordinador corregido del
workspace: conserva SHA-256 del ejecutable y configuración, reanuda el servicio
y el recolector de la sesión interactiva. Se corrigieron tres causas: `/f`
ignoraba la propiedad de coordinación; el nombre temporal del MSI no coincidía
con el origen registrado por Windows Installer; habilitar la tarea no reanudaba
la sesión actual. El origen se consulta mediante `ProductsEx/SourceListInfo`
después de verificar el paquete y se prepara en un directorio protegido.

Evidencia privada: `.local-test/acceptance-20261120/windows-repair-session.json`.
El MSI existente sigue sin firma y conserva los scripts anteriores dentro de
su payload. Antes de distribuir hay que generar una nueva entrega versionada
con estas correcciones, firmarla y repetir la aceptación del artefacto final;
no sustituir el MSI 0.2.0 existente por otro de la misma versión.

El usuario confirmó que todavía no dispone de certificado de firma. No se
generó una firma autofirmada ni se desactivó la validación de producción.
La reparación aprobada no acredita desinstalación, downgrade, reboot, múltiples
sesiones/RDP ni la recuperación de SCM después de un fallo real.

Los DEB amd64 0.2.0 de Debian 12, Ubuntu 22.04 y Ubuntu 24.04 pasaron
`tests/deb_package_lifecycle.sh`: dependencias reales resueltas por APT,
ejecución de `--version-json`, permisos de configuración, reinstalación,
eliminación y purga con conservación de configuración y archivos sintéticos
de identidad/cola. Evidencia en `.local-test/acceptance-20261120/`:
`debian12-r4.log`, `ubuntu2204-r3.log` y `ubuntu2404-r2.log`.
El administrador de servicios está simulado explícitamente en contenedores
desechables; no se acredita systemd, reboot, X11, RDP ni una sesión real Linux.
La purga conserva deliberadamente los datos; no es una eliminación de identidad.

Estado 2026-11-16: por autorización del usuario se actualizó el piloto Windows a **0.2.0**, conservando configuración y relanzando servicio/sesión. La campaña funcional sigue diferida hasta completar instaladores. No es una release estable ni está autorizado distribuir un paquete sin firma como producción.

MSI instalado: candidato corregido de `.local-test/agent-0.2.0-r2/`, sin firma, con comprobación de metadatos/hash. El primer candidato quedó conservado sin instalar en `.local-test/agent-0.2.0/`. El coordinador tolera procesos que terminan durante la inspección y restaura autostart si falla la preparación. Los directorios privados no se publican en Git.

## Formato recomendado

- Windows x64: MSI de WiX, servicio y acompañante de sesión en el mismo producto; `Instalar.cmd` abre `setup.ps1` para elegir instalar/actualizar, reparar o desinstalar y solicitar elevación. El asistente usa el coordinador `install-package.ps1`; no abrir el MSI directamente. Conservar UpgradeCode y protección de downgrade. El asistente requiere la confianza del publicador aprovisionada; no desactiva la firma silenciosamente. Para un candidato privado sin firma, el administrador debe invocar explícitamente `setup.ps1 -LocalTest -ExpectedSha256 HASH`.
- Ubuntu/Debian: DEB por distribución/arquitectura construida, gestionado con APT y systemd. No prometer que un solo binario cubre todas las distribuciones. El generador calcula dependencias dinámicas con `dpkg-shlibdeps` y falla si no puede resolverlas; agrega las herramientas usadas por mantenimiento explícitamente.
- Paquete genérico sin credenciales: configurar servidor, CA y enrolamiento posteriormente mediante las herramientas administrativas. Nunca incorporar el código de enrolamiento ni secretos del piloto al paquete público.
- Vista local existente «Mi equipo», de solo consulta; la administración del servicio y las credenciales requieren privilegios.

## Versionado y construcción

Se elige 0.2.0 por las nuevas políticas de privacidad y cambios funcionales desde 0.1.0. Cargo.toml es la fuente de verdad, Cargo.lock debe coincidir. `--version-json` informa versión, SO y arquitectura antes de leer configuración o iniciar el agente. Los generadores comparan esos datos con la versión solicitada y rechazan paquetes existentes. Ejecutar únicamente binarios propios de confianza; el hash verifica identidad de bytes, no confianza del publicador.

Windows requiere Rust fijado en el repositorio, herramientas MSVC, Python 3.10+ y WiX con Util. En PowerShell desde la raíz:

```powershell
cargo build --locked --release -p activity-monitor-agent
if ($LASTEXITCODE -ne 0) { throw 'Falló la compilación' }
$binary = (Resolve-Path target/release/activity-monitor-agent.exe).Path
$hash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
./packaging/windows/build-msi.ps1 -BinaryPath $binary -ExpectedSha256 $hash -Version 0.2.0 -OutputPath .local-test/agent-0.2.0/monitor-agent-0.2.0-windows-x64.msi
```

El MSI sin firma es un candidato de construcción. Firmar ejecutable antes de empaquetar y MSI después, con certificado organizacional y sello de tiempo; calcular hashes finales después de firmar. La confianza del publicador se aprovisiona por un canal independiente. `-ConfigTemplate` exige `-PrivatePilot`: ese resultado queda excluido de distribución pública.

Linux requiere un entorno nativo de la distribución objetivo, Python 3.10+, compilador y dependencias de desarrollo, dpkg-deb y dpkg-shlibdeps (dpkg-dev):

```bash
set -euo pipefail
cargo build --locked --release -p activity-monitor-agent
binary=target/release/activity-monitor-agent
hash=$(sha256sum "$binary" | cut -d ' ' -f 1)
python3 packaging/linux/build-deb.py --binary "$binary" --sha256 "$hash" \
  --version 0.2.0 --architecture amd64 \
  --output .local-test/agent-0.2.0/monitor-agent-0.2.0-linux-amd64.deb
```

Usar un directorio por distribución para no mezclar artefactos. El verificador usa Cargo para comprobar manifiesto/lockfile y funciona con Python 3.10 de Ubuntu 22.04, sin añadir un parser TOML. La arquitectura arm64 solo se anunciará tras construir y aceptar su binario nativo. Firma y confianza: [RELEASE_SIGNING.md](RELEASE_SIGNING.md).

## Orden hasta entrega

1. Preparar generadores, configuración/enrolamiento separados, dependencias DEB y asistente Windows. Implementados; aceptar los paquetes construidos por destino antes de distribuir.
2. Construir en checkout limpio, firmar, generar manifiesto mediante `scripts/release-manifest.py` con commit, entorno exacto y hashes de los archivos finales. No sustituir un artefacto entregado por otro de igual versión.
3. Una vez terminados los paquetes, probar instalación limpia, 0.1.0 → 0.2.0, reparación, downgrade rechazado, reversión y desinstalación. Conservar identidad, claves y colas; la purga es una operación separada y explícita.
4. Validar servicio/sesión, captura, privacidad, desconexiones y reinicio en los SO admitidos; después ejecutar el piloto integrado.
5. Publicar tag/release únicamente tras aprobación. Mantener autoactualización remota deshabilitada hasta firma y rollback comprobados.

La actualización local autorizada no sustituye la campaña final. Pendientes de firma real, paquetes finales y aceptación: no marcar este bloque como terminado solo por compilar.

## Construcción Linux desde Windows con Docker

Construcción completada de candidatos amd64 para Debian 12, Ubuntu 22.04 y Ubuntu 24.04, con SHA256 y metadatos del compilador/SO en sus respectivos directorios de salida. No instalados. Son candidatos de trabajo; reconstruir la entrega final desde checkout limpio, firmar y generar manifiesto antes de distribuir. La compilación conserva advertencias de código sin uso que no se han clasificado como fallos de ejecución.

`packaging/linux/Dockerfile` compila y empaqueta sin instalar el agente ni ejecutar tests. Exporta solo DEB, SHA256 y metadatos del entorno. Para Debian 12:

```powershell
docker build -f packaging/linux/Dockerfile --output type=local,dest=.local-test/agent-0.2.0-debian12 .
```

Para Ubuntu 24.04 añadir `--build-arg BUILD_IMAGE=ubuntu:24.04@sha256:24737cef17f4e22b1977df7d9a05e77c4e28b7da5b8911f8d8ccda136bf3573c` y `--build-arg BUILD_CACHE_ID=monitor-agent-ubuntu2404`, usando otro directorio de salida. Para Ubuntu 22.04 usar `BUILD_IMAGE=ubuntu:22.04@sha256:9abcaffc92a5279fff3a6a41635c0800ea33e528425fb389658806476e7defde` y `BUILD_CACHE_ID=monitor-agent-ubuntu2204`. No reutilizar la caché de compilación entre distribuciones. Conservar cada artefacto entregado; elegir una salida nueva en reconstrucciones. Aceptación instalada de todos los Linux y preparación de arm64 siguen pendientes.

Referencias: [versiones MSI y MajorUpgrade](https://docs.firegiant.com/wix/schema/wxs/majorupgrade/), [dependencias de bibliotecas Debian](https://www.debian.org/doc/debian-policy/ch-sharedlibs.html).
