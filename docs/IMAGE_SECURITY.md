# Escaneo de imágenes y componentes

El escaneo se realiza sobre imágenes locales identificadas por ID; no actualiza contenedores ni volúmenes. Necesita PowerShell y Docker con contenedores Linux. El script fija el digest del escáner y consulta su base de vulnerabilidades al ejecutar.

```powershell
./scripts/security-images.ps1 -Images 'monitor-security-dashboard:20261106' -FailOnHigh
```

Cada ejecución crea una carpeta privada nueva bajo `.local-test/security/images/`, con resumen, informe de vulnerabilidades y SBOM CycloneDX por imagen. No publicar automáticamente estos informes. El resumen registra referencia, ID, fecha y escáner. `-FailOnHigh` devuelve error si encuentra avisos críticos/altos; errores de análisis también interrumpen la ejecución. Sin ese parámetro el script informa hallazgos sin bloquear por severidad.

Validación del 2026-11-06: base Nginx 1.30.6 Alpine, digest `583b7f2fce1cdb8da5ad5c39ebd3c33b98b87d493190e548bdee1fa8c619c311`, y dashboard candidato completo: cero entradas de vulnerabilidad conocidas reportadas. Build productivo y prueba HTTPS/rootless/archivos sensibles/límite de login aprobados. Esto no cubre las otras imágenes ni demuestra ausencia de todas las vulnerabilidades; las dependencias JavaScript también se analizan desde su lockfile.

El candidato no reemplaza el dashboard activo. Debian bookworm actualizado todavía presenta avisos; RabbitMQ/PostgreSQL requieren revisión de versiones y migración, no sustitución automática de volúmenes. No dar por cerrado el bloque de imágenes con solo el dashboard.

Para probar otra imagen con la configuración productiva:

```powershell
$env:NGINX_TEST_IMAGE='monitor-security-dashboard:20261106'
python tests/production-nginx.py
```

La prueba crea certificados, backend y red temporales, y elimina sus contenedores al terminar. No utiliza cuentas del piloto.


Seguimiento 2026-11-08: runtime candidato Debian trixie construido localmente; no sustituye DockerServer/Dockerfile ni el piloto. Base fijada a `a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2`: escaneo registró 0 críticos, 48 altos y 184 hallazgos totales. Varios avisos no tienen versión corregida reportada. No se aprueba esta base automáticamente; faltan análisis de exposición y escaneo del runtime completo.

Resultado adicional del 2026-11-08: runtime completo `monitor-security-server:trixie-candidate`, ID `sha256:35c32b39916b6142ef63e159fb7587b5bfba5c34fa523b5d4603f48a23eb0153`, registró 0 críticos, 52 altos y 302 hallazgos totales. Informe y SBOM privados conservados; candidato no adoptado. Cifras históricas, no un escaneo nuevo del 13 de noviembre ni una evaluación de explotabilidad.
