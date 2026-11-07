# Revisión de seguridad — 2026-11-06

## Seguimiento 2026-11-07

PR #2 y PR #3 fusionados tras CI completo satisfactorio. Actualización local Lapin 4.12.3: auditoría Rust sin avisos de mantenimiento; permanece RUSTSEC-2023-0071 de rsa en el lockfile por sqlx-mysql opcional. `cargo tree --locked --target all -i rsa` no muestra ruta activa para las funciones PostgreSQL actuales. No se silencia ni se declara eliminado: reevaluar en cada cambio de features, lockfile o artefacto. El escaneo del lockfile sigue informando el aviso.

Redis retirado de los despliegues activos por ausencia de consumidores; se conserva el detector de URLs Redis en el escáner de secretos para identificar credenciales históricas. El recorrido de identidad/AMQPS con el cliente nuevo pasó contra RabbitMQ 4.2; compilación y build Windows aprobados. CI Linux del incremento sigue pendiente. La revisión inicial que sigue es evidencia histórica.

## Seguimiento del bloque 1

Actualización posterior a la revisión inicial: reqwest 0.12.28 con funciones explícitas JSON/native-tls elimina h2 0.3.27. El servidor usa dotenvy y se retiró env_logger sin uso del agente; async-executor pasa a 1.14.1. `cargo check --locked --workspace` aprobado en Windows. La elección de funciones sigue la [documentación de reqwest](https://docs.rs/reqwest/0.12.28/reqwest/).

Auditoría nueva en `.local-test/security/rust-audit-after.json`: una vulnerabilidad (rsa) y cinco avisos de mantenimiento, sin avisos de paquetes retirados. rsa está enlazado a sqlx-mysql en el lockfile/metadatos; `cargo tree --locked --target all -i rsa` no muestra una ruta activa con las funciones actuales de PostgreSQL. Esto distingue el inventario del grafo de compilación, no elimina el aviso ni acredita todos los artefactos posibles. No se añadió excepción al escáner. Las cifras de la revisión inicial que siguen abajo son la línea base, no el resultado posterior.

Seguimiento de validación: recorrido completo `tests/identity-stack.py` aprobado con las nuevas dependencias, incluyendo TLS, enrolamiento, renovación, revocación y rechazo de CA no confiable. El test de presupuesto usa una cuenta dedicada para no interferir con la matriz de permisos posterior. Lint global corregido en AdministrationPage.tsx. Pendientes: compilación Linux en CI y tratamiento restante de dependencias/imágenes. El agente instalado y los contenedores del piloto no se modificaron.

Estado: **endurecimiento implementado y comprobaciones focalizadas aprobadas; producción no aprobada**. Cambios locales sobre f6c440f, todavía no publicados ni desplegados en el piloto. Agente conserva 0.1.0. Esta revisión no constituye una prueba de penetración exhaustiva.

## Hallazgos y correcciones

| Área | Resultado de revisión / corrección | Estado |
|---|---|---|
| Autenticación | Sesión opaca, hash en DB, Argon2id, MFA obligatorio para administradores en producción, rechazo de repetición TOTP, caducidad absoluta/inactividad y revocación. | Comprobado en entorno aislado |
| Secretos en navegador | No se encontraron claves de API persistidas en el cliente revisado. Cookie HttpOnly; token CSRF en memoria, distinto de una credencial API. Se elimina almacenamiento legado de auth_token. | Revisión de código; falta inspección final del navegador desplegado |
| Cookies / CSRF | Producción usa __Host-monitor_session, Secure, HttpOnly, SameSite=Strict y Path=/. Verificación de origen y token en mutaciones. Nuevo registro de rechazos CSRF. | Matriz automatizada aprobada; piloto HTTP usa cookie de desarrollo |
| Administración | Autorización central en rutas normales y /api; cambios de rol, cuentas desactivadas y revocación restringen sesiones existentes. | Pruebas aprobadas |
| XSS | React escapa texto; no se encontraron sinks peligrosos en el código revisado. CSP bloquea scripts inline, objetos y framing. Se añadió CSP al Nginx del piloto y cabeceras a respuestas API. | Cabeceras verificadas; prueba de payloads en navegador pendiente |
| Límites | Presupuestos existentes de login y cuenta; nuevos límites por IP en Nginx (login 10/min, ráfaga 10; API 30/s, ráfaga 120), cuerpo máximo 2 MiB y tiempos de lectura. | Límites por cuenta y login Nginx comprobados; ajustar para oficinas detrás de NAT |
| Registros | Auditoría de acceso y operaciones sensibles; nuevos conteos de rechazos CSRF/accesos denegados y refresco de pantalla cada 60 s. Access log Nginx excluye query strings. | Pruebas aprobadas; alertas externas pendientes |
| Archivos expuestos | Nginx rechaza archivos ocultos, respaldos, claves, configuraciones y sourcemaps. | Rutas sensibles devuelven 404; cabeceras también presentes en errores |
| Configuración | Puertos del Compose de desarrollo y Vite limitados a localhost. Ejemplo sin contraseña inicial ni token compartido prefijados. Producción exige secretos externos, cuentas gestionadas, HTTPS/AMQPS y rol DB restringido. Rechazo nuevo de DEBUG/RUST_LOG debug/trace. No cargar dotenv cuando MONITOR_ENV ya indica producción. | Pruebas de validación de configuración aprobadas |
| Base de datos | Perfil productivo separa propietario de migraciones, aplicación y mantenimiento, y no publica PostgreSQL. | Revisión de configuración; falta verificar privilegios y aislamiento efectivos del destino final |
| Dependencias JS | Corregidos follow-redirects y minimatch; retirado autoprefixer sin uso. Ocho entradas afectadas por dos avisos antes, cero vulnerabilidades conocidas según npm audit después. | Build aprobado; lint global tiene un error pendiente en AdministrationPage.tsx |
| Dependencias Rust | Retirados env_logger y thiserror directos sin uso del servidor. | Dos avisos de vulnerabilidad todavía abiertos; ver abajo |

## Pendientes que impiden cerrar seguridad

1. **Credencial histórica:** el escaneo detectó un token GitHub en el antiguo usb/agent-config.json (commit conocido fc887c4). Ya no está en el árbol actual; su revocación no está confirmada. Borrar archivos o commits no revoca el token. Revocarlo en GitHub y revisar su uso; evaluar limpieza del historial posteriormente. No se reproduce la credencial en este informe.
2. **Rust:** RUSTSEC-2026-0258 afecta h2 0.3.27, incorporado por reqwest 0.11/hyper 0.14. La versión corregida indicada es h2 >=0.4.16, requiere migrar clientes y revisar compatibilidad; no forzar una versión incompatible. RUSTSEC-2023-0071 afecta rsa 0.9.10 en el lockfile, sin parche informado. El árbol activo Windows consultado no muestra rsa; falta determinar las features/targets que lo incorporan y eliminar la dependencia si no se usa. También hay avisos de mantenimiento (dotenv, rustls-pemfile, zbus_names/zvariant) y async-executor retirado del registro. No se han silenciado avisos.
3. **Imágenes Docker:** actualizar bases y servicios, generar imágenes nuevas y repetir escaneo antes de desplegar. Los resultados siguientes corresponden a imágenes existentes del piloto, no a una reconstrucción de estos cambios. No equivalen a exploits demostrados; son entradas paquete/aviso y pueden repetir una CVE.

| Imagen actual | Críticas | Altas | Medias | Bajas | Desconocidas |
|---|---:|---:|---:|---:|---:|
| dashboard | 1 | 6 | 2 | 44 | 0 |
| server | 2 | 49 | 139 | 151 | 2 |
| postgres | 3 | 89 | 75 | 18 | 5 |
| rabbitmq | 2 | 45 | 32 | 25 | 2 |
| redis | 0 | 5 | 26 | 16 | 2 |

4. **Operación:** desplegar y verificar TLS/cookies/permisos DB reales; configurar alertas de seguridad externas y respuesta a incidentes. Los límites de cuenta en memoria son por instancia: revisar coordinación antes de múltiples réplicas. Redis no tiene uso funcional identificado y está excluido por defecto del perfil productivo; decidir retiro del piloto.
5. **Validación final:** prueba XSS con payloads en navegador, comprobación de secretos en assets construidos, análisis de agente/IPC/instaladores y actualización firmada, revisión de dependencias de todas las plataformas y campaña final. No se certifican áreas fuera de la matriz comprobada.

## Evidencia reproducible

- `python tests/identity-stack.py --human-only`: PASS para cookies productivas, MFA/replay, roles, ambos prefijos, CSRF, expiración, revocación, exportación/operaciones de operador, auditoría sin contenido privado y límite por cuenta/cabeceras.
- `cargo test --locked -p activity-monitor-server startup_config::tests`: 4 aprobadas.
- `cargo test --locked -p activity-monitor-server api::tests`: 3 aprobadas.
- `python tests/production-nginx.py`: PASS con TLS verificado, usuario 65532, filesystem de solo lectura, capacidades retiradas, cabeceras, archivos sensibles, límite de login y canario de query ausente del access log. Backend sintético y contenedores eliminados al terminar.
- Build dashboard y servidor/probe aprobados. `npm run lint`: un error en AdministrationPage.tsx por setState síncrono en efecto; no se presenta como aprobado.
- `scripts/security-inventory.py`: 1257 blobs Git revisados; un token histórico probable real y dos coincidencias sintéticas verificadas. Sin coincidencias exactas de secretos privados del piloto. Alcance: objetos alcanzables locales de hasta 4 MiB y archivos actuales no ignorados; no cubre forks, objetos remotos inaccesibles ni prueba la validez de tokens.
- Evidencia privada en `.local-test/security/`: inventory.json, human-tests.log, npm-after.json, rust-audit.json e image-*.json. No publicar archivos privados ni valores de configuración.

Las políticas de cookies y CSP se contrastaron con las guías de [sesiones OWASP](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html) y [CSP OWASP](https://cheatsheetseries.owasp.org/cheatsheets/Content_Security_Policy_Cheat_Sheet.html). CSP complementa el escape de salida; no demuestra por sí sola ausencia de XSS.
