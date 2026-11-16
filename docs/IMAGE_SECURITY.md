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


## Candidato mínimo del servidor — 2026-11-13

El target `production` usa `gcr.io/distroless/cc-debian13:nonroot` fijado al digest
`cb87acc574abe719b6e5c547384defa2f7b1f24af9605d3f618cf852495d8be2`.
Incluye el binario y las dependencias de ejecución; se comprobó ausencia de shell,
curl y gestor de paquetes. Conserva UID/GID 65532 y el overlay de solo lectura,
capacidades eliminadas y no-new-privileges. Desarrollo conserva su target anterior.

Imagen completa local `monitor-security-server:distroless-candidate`, ID
`sha256:0a2036be1e4af6fd9444cbe5a4eb50954a652a343d16606d311cf9fa4bbd4c11`:
Trivy registró **0 críticos, 0 altos, 53 hallazgos totales**. Informe y SBOM privados
conservados. No se suprimieron avisos; menor severidad requiere clasificación y el
resultado no certifica ausencia de vulnerabilidades. El candidato trixie anterior
reportaba 52 altos; no fue adoptado. PostgreSQL/RabbitMQ y la aceptación del conjunto
siguen sus propios pendientes.

La salud se consulta con `/usr/local/bin/activity-monitor-server --healthcheck`:
solo loopback, timeout de 3 segundos, sin proxy ni redirecciones; código 0 únicamente
con HTTP 200 de `/ready`. La herramienta de cuentas acepta `--docker-image` para
calcular Argon2id en un contenedor temporal sin red ni shell. No instalar herramientas
interactivamente en la imagen productiva; usar logs e inspección desde el host.

Antes de promover, verificar procedencia/firma del digest con el procedimiento
[oficial de Distroless](https://github.com/GoogleContainerTools/distroless#how-do-i-verify-distroless-images),
repetir el escaneo del artefacto definitivo y validar HTTPS/AMQPS y despliegue completo.
La nueva prueba CI cubre estructura, hash de contraseña, salud y arranque con
DB/broker de ensayo; no reemplaza la aceptación TLS del destino.
