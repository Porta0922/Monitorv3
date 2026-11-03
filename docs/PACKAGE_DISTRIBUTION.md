> Actualización 2026-10-28: el cierre de implementación, los nuevos flujos multiusuario
> y la integración de paquetes están descritos en [P2_IMPLEMENTATION_CLOSURE.md](P2_IMPLEMENTATION_CLOSURE.md).
> Las limitaciones y pendientes de esta página anteriores a esa fecha son históricos;
> la validación funcional continúa en P4.

# Distribución del agente

Se agregaron generadores de paquetes desde artefactos ya compilados. No descargan ni compilan el agente. Los scripts y definiciones están escritos, pero no se generaron ni instalaron paquetes en esta fase.

## Linux

python3 packaging/linux/build-deb.py --binary RUTA --sha256 HASH --version 3.4.0 --architecture amd64 --output RUTA.deb

El DEB instala binario, unidad systemd, herramientas administrativas y acceso «Monitor — Mi equipo». Crea cuenta de servicio, conserva configuración/identidad/colas y deja el servicio detenido. Configuración generada solo si no existe; no incluye credenciales reales. remove/purge no borran secretos ni datos pendientes. Prerm detiene servicio y retira el autostart de sesión existente. Después de upgrade, el autostart requiere habilitación explícita.

Dependencias declaradas para Debian 12/Ubuntu 22.04 o posteriores; no equivale a acreditar todas las versiones/arquitecturas. Falta comprobar dependencias dinámicas de los artefactos finales en P4.

update-signed.sh BINARY SIGNATURE verifica firma SHA256 mediante OpenSSL y /etc/activity-monitor/release-public.pem, que debe aprovisionar el administrador por un canal independiente. No confía en una clave incluida junto al candidato. La verificación cubre los mismos bytes que después se entregan al actualizador por hash. La generación/firma del repositorio APT queda fuera de este script.

update-service.sh --recover restaura el binario anterior de una operación interrumpida y deja el servicio detenido. El journal y la copia contienen hashes y se conservan si falla la recuperación. No altera base local ni configuración. Las copias de rollback requieren mantenimiento explícito.

## Windows

build-msi.ps1 -BinaryPath RUTA.exe -ExpectedSha256 HASH -Version 3.4.0 -OutputPath RUTA.msi

Requiere WiX con WixToolset.Util.wixext instalado en el entorno de construcción. Define servicio LocalSystem, recuperación, stop/remove de Windows Installer, actualización mayor, configuración permanente y acceso local. Incluye herramientas administrativas de sesiones. El MSI generado es inicialmente sin firma: no es una release lista para distribución.

install-package.ps1 valida Authenticode y el thumbprint aprovisionado en ProgramData/ActivityMonitor/release-publisher.txt antes de llamar a Windows Installer. update-signed.ps1 aplica el mismo control a un ejecutable local. No hay descarga remota. El certificado/clave privada de publicación debe pertenecer a la organización; no se genera ni se sustituye automáticamente.

update-service.ps1 -Recover restaura un binario anterior desde journal/copia verificados y deja el servicio detenido. MSI administra su propio rollback; no confundirlo con rollback del script de actualización de ejecutables.

Pendiente de integración: el alta/baja automática de tareas de sesión dentro de las transacciones MSI. Antes de desinstalar un MSI, retirar autostart con session-autostart.ps1 disable; no se afirma aún que MSI resuelva por sí solo toda la sesión.

## Alcance pendiente

No se cierran P2 ni la entrega completa solo por tener definiciones de paquete. Faltan integración MSI/sesiones, migración de instalaciones antiguas, purga explícita y multiusuario/RDP; firmas reales y ejecución de instaladores/rollback corresponden a P4 una vez completado el código.

Referencias de autoría: [WiX ServiceInstall](https://docs.firegiant.com/wix/schema/wxs/serviceinstall/), [ServiceControl](https://docs.firegiant.com/wix/schema/wxs/servicecontrol/) y [ServiceConfig](https://docs.firegiant.com/wix/schema/util/serviceconfig/).
