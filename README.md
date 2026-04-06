# ActivityMonitor Enterprise v3

Sistema empresarial de monitoreo de actividad con arquitectura distribuida:

- Agente multiplataforma en Rust (Windows/Linux/macOS)
- Backend en Rust (Axum + RabbitMQ + PostgreSQL/TimescaleDB)
- Dashboard web en React + TypeScript

Incluye monitoreo de aplicaciones en foco, actividad/inactividad, teclado/mouse, USB, inventario de software y eventos de WiFi por dispositivo.

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

## Arquitectura

1. El agente captura eventos en el endpoint.
2. Publica eventos en RabbitMQ (exchange topic `monitoring`).
3. El servidor consume colas (`monitoring.activity`, `monitoring.heartbeat`, `monitoring.usb`, `monitoring.inventory`, `monitoring.wifi`).
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