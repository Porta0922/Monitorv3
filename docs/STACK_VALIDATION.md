# Validación del stack refactorizado

El servidor actualizado se compiló y desplegó en el stack aislado el 2026-10-26.
El smoke de RabbitMQ, deduplicación, API, Nginx, métricas diarias y mantenimiento aprobó.
El backend usa Rust 1.99.1,
Cargo.lock y `--locked`; el frontend usa `npm ci`. No requiere Rust en el host
para desplegar. `rust-toolchain.toml` fija también la versión de desarrollo.

El dashboard utiliza `/api` en su propio origen. Nginx reenvía las peticiones
al servicio `server:3000`; Vite usa un proxy equivalente en desarrollo.
El backend conserva las rutas antiguas sin prefijo para agentes existentes.

## Pruebas en un stack descartable

```sh
docker build --target validation -f DockerServer/Dockerfile -t monitor-validation .
docker compose -p monitor-refactor-test -f docker-compose.yml -f tests/compose.smoke.yml up -d --build --wait --wait-timeout 180
```

El override limita los puertos de prueba a loopback y evita el puerto 3000:
dashboard 15173, API 13000, PostgreSQL 15432, AMQP 15673 y administración RabbitMQ 15674.

En PowerShell:

```powershell
$env:SMOKE_BASE_URL='http://127.0.0.1:15173'
$env:SMOKE_BROKER_URL='http://127.0.0.1:15674'
python tests/smoke_stack.py
```

En Linux:

```sh
SMOKE_BASE_URL=http://127.0.0.1:15173 SMOKE_BROKER_URL=http://127.0.0.1:15674 python3 tests/smoke_stack.py
```

El test crea un dispositivo sintético y publica dos veces el mismo evento.
Verifica login a través de Nginx, registro por RabbitMQ, rutas parametrizadas,
una sola muestra de CPU=42.5%, resumen y consulta de alertas. No instala un agente
en esta computadora ni prueba sus recolectores del sistema operativo.

Solo para eliminar los recursos de prueba creados con ese nombre:

```sh
docker compose -p monitor-refactor-test -f docker-compose.yml -f tests/compose.smoke.yml down --volumes
```

## Alcance verificado localmente

- Arranque completo con TimescaleDB, RabbitMQ, Redis, API, dashboard y mantenimiento.
- Recorrido de una métrica sintética desde RabbitMQ hasta la API a través de Nginx.
- Persistencia de la muestra y recepción de nuevas muestras tras reiniciar DB/API.
- Mantenimiento periódico ejecutado por Compose.

Las cinco pruebas unitarias Rust pendientes pasaron al recuperar Docker.
Tras incorporar migraciones versionadas, pasaron las seis pruebas Rust
(incluida la integración de instalación vacía, actualización de esquema antiguo
con datos y arranques simultáneos) y las cuatro pruebas de retención.
Registro local: `migration-validation.log`. Ver [migraciones](MIGRATIONS.md).
La integración usó una instancia temporal; el stack ya en ejecución no se actualizó.
El workflow remoto tampoco fue ejecutado todavía.

Esto no certifica los servicios instalados Windows/Linux ni todos los fallos durante
un commit. Deduplicación y escrituras ahora comparten transacción y el ACK ocurre
después del commit; ver TELEMETRY_DELIVERY.md. Las pruebas sintéticas de migración
incluyen chunks comprimidos, pero no una copia real de producción. Consultar TODO.md.
