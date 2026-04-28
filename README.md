# ActivityMonitor Enterprise v3
*Actualizado: 28 de Abril, 2026*

Sistema empresarial de monitoreo de actividad y seguridad con arquitectura distribuida:

- Agente multiplataforma en Rust (Windows/Linux/macOS)
- **Despliegue Híbrido (Windows)**: Servicio de sistema (Sesión 0) + Agente de usuario para captura de actividad.
- Backend en Rust (Axum + RabbitMQ + PostgreSQL/TimescaleDB)
- Dashboard web en React + TypeScript

Incluye monitoreo de aplicaciones en foco, actividad/inactividad, teclado/mouse (heatmaps), USB, inventario de software, eventos de WiFi y detección de amenazas mapeadas al framework MITRE ATT&CK mediante osquery.

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
- **Seguridad MITRE ATT&CK**: detección de amenazas vía osquery con 11 técnicas mapeadas.
- **Detección de copia en USB**: identifica archivos recientemente escritos en drives USB (T1052.001) con deduplicación SHA-256.
- **Heatmaps de Actividad**: Seguimiento de intensidad de uso de teclado y mouse por hora.
- **Despliegue como Servicio**: Integración nativa con Windows Service Manager (SCM) para persistencia.
- **Cache offline**: Almacenamiento local en SQLite para entornos sin conectividad inmediata.
- Busqueda y filtrado de dispositivos por MAC address, hostname, apodo o Device ID.

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
*El instalador ofrece opciones para instalación como Servicio, Tarea de Usuario o Híbrida.*

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

El agente ejecuta queries osquery con **scheduler inteligente por-query** (si osqueryi esta instalado; de lo contrario opera en modo silencioso sin afectar el resto del agente).

Cada query ejecuta según su propia frecuencia para optimizar recursos:

| Query | Intervalo | Tecnica MITRE | Severidad |
|---|---|---|---|
| `powershell_encoded_commands` | 5min | T1059.001 — PowerShell Encoding | HIGH |
| `powershell_download_cradles` | 5min | T1105 — Download Cradles | HIGH |
| `suspicious_script_hosts` | 5min | T1059.005 — Script Hosts | MEDIUM |
| `unusual_listening_ports` | 10min | T1021 — Remote Services | MEDIUM |
| `executable_in_temp_paths` | 10min | T1105 — Ingress Tool Transfer | HIGH |
| `lolbins_with_remote_content` | 10min | T1218 — LOLBins | HIGH |
| `scheduled_tasks_hidden` | 20min | T1053.005 — Scheduled Task | HIGH |
| `autorun_registry_keys` | 20min | T1547.001 — Registry Run Keys | MEDIUM |
| `cmd_spawned_by_unusual_parent` | 20min | T1059.003 — Command Shell | MEDIUM |
| `unsigned_system_path_processes` | 30min | T1036 — Masquerading | MEDIUM |
| `startup_items_persistence` | 30min | T1547.009 — Startup Items | LOW |

### Configuración del scheduler osquery

Por defecto, el scheduler está **deshabilitado** (no consume recursos). Para activarlo:

```bash
# Windows
set AGENT_OSQUERY_SCHEDULER_SECONDS=30

# Linux/macOS
export AGENT_OSQUERY_SCHEDULER_SECONDS=30
```

Valores recomendados:
- `0` o no establecida = deshabilitado (default)
- `30` = tick cada 30s, cada query ejecuta segun su intervalo
- `60` = tick cada 60s (menos aggressive)

### Control central desde servidor (recomendado para flotas)

Ahora el agente puede tomar la política de osquery desde el servidor usando:

```bash
# En el agente
set AGENT_SERVER_URL=http://TU-SERVIDOR:3000
set AGENT_AUTH_TOKEN=dev-agent-token
```

Endpoint utilizado por el agente:
- `GET /api/agent/osquery-policy?device_id=<uuid>`
- Header: `x-agent-token: <AGENT_AUTH_TOKEN>`

Variables de entorno del servidor para controlar la política global:

```bash
OSQUERY_POLICY_PROFILE=balanced
OSQUERY_POLICY_TICK_SECONDS=60
AGENT_AUTH_TOKEN=dev-agent-token
```

Perfiles soportados:
- `off` -> desactiva osquery
- `slow` -> tick default 90s
- `balanced` -> tick default 60s
- `aggressive` -> tick default 30s

Notas de rendimiento:
- El agente aplica limites de seguridad (`min 30s`, `max 900s`) para evitar sobrecarga accidental.
- Si el servidor no responde, el agente vuelve automaticamente a `AGENT_OSQUERY_SCHEDULER_SECONDS` local.

### Detección de copia en USB (T1052.001)

El agente monitorea continuamente drives USB removibles cada **45 segundos** y detecta:
- Archivos recientemente escritos (últimos 180s)
- Máximo 20 archivos por drive
- Deduplicación por SHA-256 para evitar alertas duplicadas

Los hallazgos se publican a `monitoring.security` con severidad **HIGH** cuando se detectan escrituras en USB.

### Integración con dashboard

Los hallazgos se publican a la cola `monitoring.security`, el servidor los persiste en la tabla `security_events` y el dashboard los muestra en la pestaña **Seguridad** con:

- Tarjetas de resumen (alertas hoy, criticas, dispositivos afectados, tecnica mas frecuente)
- Filtros por rango de fechas, severidad y tecnica MITRE
- Enlace directo a `attack.mitre.org` por cada tecnica
- Vista expandida del JSON `raw_data` de cada evento
- Búsqueda por MAC address del dispositivo

## Documentacion del proyecto

Consulta estos archivos para detalle tecnico y operativo:

- `START_HERE.md`
- `ARCHITECTURE.md`
- `API_REFERENCE.md`
- `DIAGNOSTIC_CHECKLIST.md`
- `WINDOWS_DEMO_GUIDE.md`
- `WINDOWS_DEMO_GUIDE_ES.md`

## Notas operativas

- **osquery opcional**: se descarga/instala automáticamente en Windows (`deploy/install-windows.bat`). Si no está disponible, el agente funciona normalmente sin scans de seguridad.
- **Scheduler deshabilitado por defecto**: establece `AGENT_OSQUERY_SCHEDULER_SECONDS` para activar análisis de amenazas.
- **USB detection siempre activo**: monitorea 24/7 si está habilitado el agente; no requiere configuración adicional.
- **Deduplicación SHA-256**: la tabla `security_events` usa fingerprints para evitar duplicados, incluso si una query detecta lo mismo múltiples veces.
- Si las metricas del dia aparecen infladas, valida colas RabbitMQ y timestamps del evento.
- Para diagnostico rapido en Windows, usa `diagnostic.ps1`.
- Para verificar RabbitMQ, usa `verify_rabbitmq.ps1`.