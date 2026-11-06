# Preparación de GitHub — 2026-11-03

## Autorización permanente — 2026-11-06

El usuario autoriza crear ramas, commits y PR en Monitorv3, publicar avances comprobados y fusionarlos cuando pase el CI. No reescribir historial ni publicar secretos. Mantener las restricciones del repositorio; publicar código no autoriza declarar producción ni distribuir paquetes privados. La conexión GitHub tiene permiso de escritura comprobado. La revocación del token histórico continúa pendiente del titular.

## Estado

Código piloto 0.1.0 comprobado en Windows y stack local. El 2026-11-03 el usuario autorizó actualizar el repositorio existente Monitorv3 con el árbol limpio y conservar su historial. La revocación de credenciales heredadas queda pendiente por decisión expresa del usuario; no se considera realizada. Producción no aprobada. Publicar como trabajo en desarrollo; no anunciar Ubuntu/Debian, firma o recuperación completa como certificados.

## Comprobaciones preparadas

- `python scripts/check-publication.py`: revisa archivos versionados y nuevos no ignorados, artefactos privados, formatos reconocibles de credenciales y coincidencias exactas con secretos del piloto si están disponibles localmente. Solo imprime rutas y motivos; no valores. No revisa historial, vulnerabilidades ni todos los tipos de secretos.
- `.local-test`, `.production`, herramientas, paquetes, bases locales y claves fuera del conjunto publicable. El ejecutable antiguo de USB fue retirado del índice y permanece en disco, ignorado.
- Login de las comprobaciones adaptado a cookies/CSRF; smoke aislado aprobado. Recursos de UI incluidos en Dockerfiles Linux; ejecución completa Linux pendiente.
- Tags de releases: `agent-vVERSION`, coincidencia obligatoria con Cargo.toml, build locked, hashes y release en borrador; serie 0.x marcada prerelease. No genera paquetes firmados.

## Credenciales heredadas pendientes

Se detectó un token GitHub fine-grained en `usb/agent-config.json`, presente en fc887c4 (2026-06-24). Retirado del árbol actual; el historial sigue intacto. No se intentó usar el token ni comprobar su vigencia contra GitHub. La fecha del commit no demuestra fecha de creación. El usuario debe identificarlo y revocarlo en Settings → Developer settings → Personal access tokens → Fine-grained tokens. No pegar tokens en chats o issues.

También se retiraron credenciales antiguas de RabbitMQ de los instaladores heredados. El piloto nuevo usa secretos diferentes; si la instalación antigua continúa activa, rotar sus credenciales. Los instaladores heredados quedaron con marcadores y no son el método vigente: usar `packaging/` y el README.

Eliminar un secreto del árbol no lo invalida ni lo elimina del historial. Primero revocar/rotar. Una eventual reescritura del historial debe planificarse separadamente; no se autoriza por preparar el código. Referencia: https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository

## Decisión de publicación

Actualizar `Porta0922/Monitorv3`, rama `main`, mediante commit normal. No crear repositorio nuevo, no borrar el remoto, no reescribir historial ni usar force-push. La rama remota se verifica antes de subir para conservar cambios ajenos. No crear tag ni release productiva.

El usuario autorizó continuar con la limpieza del árbol actual aunque la credencial permanezca en commits anteriores. Esto no revoca el token ni elimina su exposición: sigue siendo necesario revocarlo y rotar las credenciales antiguas que continúen activas. Las configuraciones privadas del piloto nuevo nunca forman parte del commit.

Antes de la subida se ejecutan preflight y revisión del índice. Después se comprueba el SHA remoto y se consulta CI; jobs en curso no se presentan como aprobados. El TODO de producción permanece abierto.

## Pendientes de aceptación

Reinicio completo de Windows, desconexión prolongada, Linux nativo, restauración, firmas, permisos/UX completa y seguridad productiva. El ensayo de SCM y el corte RabbitMQ acotado ya tienen evidencia en REFACTOR_PROGRESS.md.

## Revisión adicional del contenido

El control automático rechazó referencias de red interna en instaladores heredados. Se sustituyeron por `monitor.example.invalid` y usuarios de ejemplo; el preflight impide reintroducir la dirección detectada. No se cambió la configuración privada del piloto.
