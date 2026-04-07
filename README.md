# ActivityMonitor Enterprise v3

Sistema empresarial de monitoreo de actividad y seguridad con arquitectura distribuida:

- Agente multiplataforma en Rust (Windows/Linux/macOS)
- Backend en Rust (Axum + RabbitMQ + PostgreSQL/TimescaleDB)
- Dashboard web en React + TypeScript

Incluye monitoreo de aplicaciones en foco, actividad/inactividad, teclado/mouse, USB, inventario de software, eventos de WiFi y detección de amenazas mapeadas al framework MITRE ATT&CK mediante osquery.

## Caracteristicas principales

- Monitoreo de actividad en tiempo casi real
- Registro de tiempo activo/inactivo por dispositivo
- Contadores de entrada: teclas, movimientos y clicks
- Historial de USB (conexiones/desconexiones)
- Inventario de software instalado
- Historial de red WiFi (SSID/BSSID/señal/estado)
- Cola de mensajes con RabbitMQ y persistencia en PostgreSQL
- Dashboard con vistas por dispositivo y exportacion CSV
- Cache offline en agente con reintentos de sincronizacion
- **Seguridad MITRE ATT&CK**: deteccion de amenazas via osquery con 8 tecnicas mapeadas
- Deduplicacion de eventos de seguridad por fingerprint SHA-256
- Busqueda de dispositivos por MAC address, hostname, apodo o Device ID

## Arquitectura

1. El agente captura eventos en el endpoint.
2. Publica eventos en RabbitMQ (exchange topic `monitoring`).
3. El servidor consume colas (`monitoring.activity`, `monitoring.heartbeat`, `monitoring.usb`, `monitoring.inventory`, `monitoring.wifi`, `monitoring.security`).
4. Persiste datos en PostgreSQL/TimescaleDB.
5. El dashboard consulta API REST para mostrar metricas e historicos.

## Estructura del repositorio

```text
.
├── agent/          # Cliente de captura (Rust)
├── server/         # API y consumidor RabbitMQ (Rust)
├── dashboard/      # Frontend React + Vite + TS
├── migrations/     # Scripts SQL de esquema
├── deploy/         # Scripts de instalacion por OS
└── docs/           # Documentacion adicional
```

## Requisitos

- Rust 1.70+
- Node.js 18+
- npm 9+
- Docker + Docker Compose

## Inicio rapido

### 1) Levantar infraestructura

Desde la raiz del repo:

```bash
docker compose up -d
```

Servicios esperados:

- PostgreSQL/TimescaleDB: `localhost:5432`
- RabbitMQ AMQP: `localhost:5672`
- RabbitMQ UI: `http://localhost:15672` (guest/guest)
- Redis: `localhost:6379`

### 2) Compilar y ejecutar el servidor

```bash
cd server
cargo run
```

API por defecto: `http://localhost:3000`

Healthcheck:

```bash
curl http://localhost:3000/api/health
```

### 3) Compilar y ejecutar el dashboard

```bash
cd dashboard
npm install
npm run dev
```

Dashboard por defecto: `http://localhost:5173`

### 4) Ejecutar el agente

Opcion Windows:

```bat
deploy\install-windows.bat
```

Opciones Linux/macOS:

```bash
bash deploy/install-linux.sh
# o
bash deploy/install-macos.sh
```

## Build de produccion

Servidor:

```bash
cd server
cargo build --release
```

Agente:

```bash
cd agent
cargo build --release
```

Dashboard:

```bash
cd dashboard
npm run build
```

## Endpoints API (resumen)

- `GET /api/health`
- `GET /api/devices`
- `GET /api/devices/:id`
- `GET /api/logs`
- `GET /api/hourly`
- `GET /api/overview`
- `GET /api/usb`
- `GET /api/wifi`
- `GET /api/security` — eventos de seguridad con filtros opcionales
- `GET /api/security/summary` — resumen por severidad y tecnica MITRE
- `GET /api/security/:device_id` — eventos de seguridad por dispositivo

### Parametros de filtro para `/api/security`

| Parametro | Descripcion |
|---|---|
| `device_id` | UUID del dispositivo |
| `severity` | `LOW`, `MEDIUM`, `HIGH` o `CRITICAL` |
| `mitre_technique` | Ej: `T1053.005` |
| `from` | Timestamp ISO-8601 inicio |
| `to` | Timestamp ISO-8601 fin |
| `hours` | Ultimas N horas (alternativa a from/to) |
| `limit` | Maximo de resultados (default 500) |

## Modulo de seguridad (osquery + MITRE ATT&CK)

El agente ejecuta queries osquery cada **5 minutos** (si osqueryi esta instalado; de lo contrario opera en modo silencioso sin afectar el resto del agente).

| Query | Tecnica MITRE | Severidad |
|---|---|---|
| `scheduled_tasks_hidden` | T1053.005 — Scheduled Task | HIGH |
| `autorun_registry_keys` | T1547.001 — Registry Run Keys | MEDIUM |
| `powershell_encoded_commands` | T1059.001 — PowerShell | HIGH |
| `unsigned_system_path_processes` | T1036 — Masquerading | MEDIUM |
| `executable_in_temp_paths` | T1105 — Ingress Tool Transfer | HIGH |
| `unusual_listening_ports` | T1021 — Remote Services | MEDIUM |
| `startup_items` | T1547.009 — Shortcut Modification | LOW |
| `cmd_spawned_by_unusual_parent` | T1059.003 — Command Shell | MEDIUM |

Los hallazgos se publican a la cola `monitoring.security`, el servidor los persiste en la tabla `security_events` y el dashboard los muestra en la pestaña **Seguridad** con:

- Tarjetas de resumen (alertas hoy, criticas, dispositivos afectados, tecnica mas frecuente)
- Filtros por rango de fechas, severidad y tecnica MITRE
- Enlace directo a `attack.mitre.org` por cada tecnica
- Vista expandida del JSON `raw_data` de cada evento

## Documentacion del proyecto

Consulta estos archivos para detalle tecnico y operativo:

- `START_HERE.md`
- `ARCHITECTURE.md`
- `API_REFERENCE.md`
- `DIAGNOSTIC_CHECKLIST.md`
- `WINDOWS_DEMO_GUIDE.md`
- `WINDOWS_DEMO_GUIDE_ES.md`

## Notas operativas

- Si las metricas del dia aparecen infladas, valida colas RabbitMQ y timestamps del evento.
- Para diagnostico rapido en Windows, usa `diagnostic.ps1`.
- Para verificar RabbitMQ, usa `verify_rabbitmq.ps1`.
- osquery es **opcional**: si no esta instalado, el agente inicia igual y omite los scans de seguridad.
- La tabla `security_events` usa deduplicacion por fingerprint SHA-256; eventos repetidos no se duplican en la base de datos.