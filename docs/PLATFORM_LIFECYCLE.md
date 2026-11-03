# Operación administrativa del agente — 2026-10-27

Scripts locales, todavía no paquetes MSI/DEB. Requieren administrador/root y un
artefacto obtenido por un canal confiable. El SHA256 esperado verifica integridad;
no reemplaza una firma de release. No descargar hashes del mismo origen sin verificar.

## Linux

```sh
sudo sh packaging/linux/install-service.sh /ruta/agente /ruta/agent.env
sudo systemctl start activity-monitor-agent.service
sudo sh packaging/linux/update-service.sh /ruta/agente-nuevo SHA256_ESPERADO
sudo sh packaging/linux/uninstall-service.sh
```

Instalación inicial habilita arranque pero deja detenido. Actualización conserva
configuración/identidad/caché, verifica el candidato antes de detener, reemplaza
en el mismo filesystem y vuelve al binario anterior si falla el arranque o la
comprobación de estado a los 3 segundos. Si estaba detenido, permanece detenido.
Un flock excluye operaciones simultáneas. No hay rollback garantizado ante corte
de energía; la transacción del instalador todavía no tiene journal recuperable.

## Windows (PowerShell elevado)

```powershell
./packaging/windows/register-service.ps1 -BinaryPath C:\Monitor\activity-monitor-agent.exe -ConfigPath C:\Monitor\agent.env
./packaging/windows/update-service.ps1 -BinaryPath C:\Artifacts\activity-monitor-agent.exe -ExpectedSha256 SHA256_ESPERADO
./packaging/windows/uninstall-service.ps1 -BinaryPath C:\Monitor\activity-monitor-agent.exe
```

Registro requiere binario/config en directorios administrados. Actualización exige
la ruta registrada entre comillas, preserva ACL del binario con File.Replace y
restaura el anterior ante fallo de arranque. Conserva la copia .update-*.previous
para recuperación manual; no elimina copias antiguas automáticamente. Un mutex
común excluye registro/actualización/desinstalación concurrentes.

Desinstalación valida que BinaryPath coincida con el servicio y elimina únicamente
ese ejecutable y el servicio. No mata procesos por nombre ni borra directorios.

## Datos, sesiones y límites

Ambas plataformas conservan identidad, cola pendiente, configuración y logs al
quitar el servicio. Las sesiones de usuario se administran por separado. No hay
purga implícita. Para reparar/reinstalar el servicio administrado conservando identidad:

```sh
sudo python3 packaging/linux/repair-service.py --binary /ruta/agente --sha256 SHA256_ESPERADO
```

```powershell
./packaging/windows/register-service.ps1 -Repair -BinaryPath C:\Monitor\activity-monitor-agent.exe -ConfigPath C:\ProgramData\ActivityMonitor\.env
```

La reparación deja el agente detenido. Linux exige cuenta/directorios existentes y rechaza unidades personalizadas; no es migración de versiones antiguas. Windows conserva rutas registradas y rechaza entornos personalizados que podría sobrescribir. Una instalación nueva sigue usando los scripts originales.

Sesiones: ver [SESSION_LIFECYCLE.md](SESSION_LIFECYCLE.md) para activación, rotación, reparación recuperable, autostart y retirada. Empaquetado MSI/DEB y firmas siguen pendientes.

Se retiraron el actualizador automático del repositorio antiguo y el batch de
autodesinstalación destructiva. Comandos remotos/web devuelven operación no disponible;
no descargan ni ejecutan instaladores. La actualización automática volverá con
releases firmadas, permisos y rollback probados.

Pruebas: Linux pasó instalación, hash incorrecto, actualización, fallo de arranque
con rollback, respeto de estado detenido y eliminación conservando datos en
contenedores Debian/Ubuntu. systemctl fue simulado. Windows pasó filesystem real
con SCM simulado, incluyendo rechazo de ruta incorrecta. Se corrigió la conversión
de null de PowerShell en File.Replace usando NullString.Value. Esto no acredita
instalación real en SCM/systemd, sesiones, firmas ni salud completa del agente.
