# ActivityMonitor Enterprise v3 — Resumen del Proyecto

> Generado el 2026-06-19 para dar contexto inmediato al trabajar en el código.

---

## 1. Propósito

Sistema de monitoreo de actividad de equipos (endpoint monitoring). Captura en segundo plano ventanas activas, apps abiertas, dispositivos USB, redes WiFi, inventario de software, eventos de seguridad (osquery), heatmaps de entrada (teclado/ratón) y uso de recursos. Todo cifrado en reposo y transmitido vía RabbitMQ.

---

## 2. Stack Tecnológico

| Componente | Lenguaje | Framework | Base de datos | Mensajería |
|-----------|----------|-----------|---------------|------------|
| **Agent** | Rust 2021 | Tokio (async) | SQLite (offline cache) + AES-256-GCM | RabbitMQ (lapin) + HTTP |
| **Server** | Rust 2021 | Axum 0.7 | PostgreSQL (sqlx 0.7) | RabbitMQ (lapin) |
| **Dashboard** | TypeScript 5.9 | React 19 + Vite 8 | — | HTTP API |
| **Infra** | Docker | docker-compose | TimescaleDB / Redis | RabbitMQ 3.12 |

---

## 3. Arquitectura

```
┌──────────────────────────────────────────────────────────────────┐
│                        DOCKER COMPOSE                            │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────────┐ │
│  │ RabbitMQ │◄──│  Agent   │   │  Server  │──►│ PostgreSQL   │ │
│  │ :5672    │   │ (Rust)   │   │ :3000    │   │ :5432        │ │
│  │ :15672   │   │          │   │ (Axum)   │   │ (TimescaleDB)│ │
│  └────┬─────┘   └──────────┘   └────┬─────┘   └──────────────┘ │
│       │                             │                           │
│       └─────── Consume ────────────┘                           │
│                                     │                           │
│                            ┌────────▼────────┐                 │
│                            │   Dashboard     │                 │
│                            │   :5173         │                 │
│                            │ (React/Vite)    │                 │
│                            └─────────────────┘                 │
└──────────────────────────────────────────────────────────────────┘
```

### Flujo de datos

1. **Agent** publica eventos al exchange `monitoring` (topic) en RabbitMQ
2. **Server** consume 7 colas (activity, heartbeat, inventory, usb, wifi, running_apps, security)
3. **Server** persiste en PostgreSQL con deduplicación (SHA-256 de `device_id|event_type|timestamp|payload`)
4. **Dashboard** consulta vía HTTP REST a `http://server:3000/api`
5. **Offline cache**: si RabbitMQ no está disponible, el agente guarda en SQLite cifrado y sincroniza después

---

## 4. Estructura del Repositorio

```
agent/                    # Agente de monitoreo (Rust)
├── Cargo.toml            # Dependencias (tokio, lapin, rusqlite, sysinfo...)
├── src/
│   ├── main.rs           # Entry point (servicio Windows / consola)
│   ├── tasks/            # 12 tareas asíncronas de monitoreo
│   │   ├── mod.rs        # TaskContext, EventMetadata, publish_or_cache
│   │   ├── window_activity.rs    # Ventana activa (c/2s)
│   │   ├── heartbeat.rs          # Heartbeat + idle (c/60s)
│   │   ├── usb_detector.rs       # USB connect/disconnect (c/60s)
│   │   ├── usb_copy.rs           # Copia de archivos a USB (Windows, c/60s)
│   │   ├── wifi_history.rs       # Estado WiFi (c/120s)
│   │   ├── running_apps.rs       # Apps abiertas (c/60s)
│   │   ├── inventory.rs          # Inventario software (30 días)
│   │   ├── heatmap.rs            # Heatmap teclado/ratón (c/3600s)
│   │   ├── resource_logger.rs    # CPU/RAM/input (c/60s)
│   │   ├── security_osquery.rs   # Escaneos osquery (cuando idle)
│   │   └── support.rs            # Reconector RabbitMQ, sincronizador cache
│   ├── monitoring.rs             # Captura ventanas, recursos (plataforma-específico)
│   ├── offline_cache.rs          # SQLite + AES-256-GCM
│   ├── inventory.rs              # Scanner de software instalado
│   ├── device_id.rs              # UUID persistente del dispositivo
│   ├── rabbitmq_publisher.rs     # Publicación RabbitMQ
│   ├── usb_detection.rs          # Detección USB multi-plataforma
│   ├── usb_file_copy_detection.rs # Detección copia a USB (Windows)
│   ├── wifi_detection.rs         # Detección WiFi multi-plataforma
│   ├── input_tracking.rs         # Heatmap de entrada (grid)
│   ├── keystroke_tracker.rs      # Contadores teclado/ratón + idle
│   ├── process_protection.rs     # Anti-kill (Job Object, prctl)
│   ├── osquery_runner.rs         # Runner de queries osquery
│   ├── config_manager.rs         # Config dinámica + remote policy
│   ├── task_supervisor.rs        # Supervisor con auto-restart
│   ├── health_reporter.rs        # Reporte de salud (c/300s)
│   ├── command_channel.rs        # Comandos remotos (HTTP long-pull)
│   ├── remote_policy.rs          # Política remota (HTTP polling)
│   └── discovery.rs              # Auto-descubrimiento config
│
server/                   # Servidor backend (Rust/Axum)
├── Cargo.toml            # Dependencias (axum, sqlx, lapin, jsonwebtoken...)
├── src/
│   ├── main.rs           # Entry point + init
│   ├── api.rs            # AppState + create_router (mountea 7 dominios + CORS)
│   ├── auth.rs           # JWT + Argon2id (NO usado en rutas)
│   ├── config.rs         # RuntimeConfig (desde env vars)
│   ├── postgres_db.rs    # ~2071 líneas: schema + CRUDs
│   ├── rabbitmq_consumer.rs # Consumidor 7 colas RabbitMQ
│   ├── ws.rs             # WebSocket (NO integrado, dead code)
│   └── domains/
│       ├── mod.rs
│       ├── shared.rs     # Tipos compartidos (filtros, format_duration)
│       ├── device/       # models OK, routes EMPTY, repo EMPTY
│       ├── activity/     # models OK, routes EMPTY, repo partial
│       ├── inventory/    # models OK, routes EMPTY, repo partial
│       ├── usb/          # models OK, routes EMPTY, repo partial
│       ├── wifi/         # models OK, routes OK (2 endpoints), repo partial
│       ├── security/     # models OK, routes EMPTY, repo FULL (duplica postgres_db)
│       ├── keystroke/    # TODO empty (models, routes, repo vacíos)
│       └── agent/        # routes OK (4 endpoints: policy, commands, osquery)
│
dashboard/                # Frontend React (TypeScript/Vite/Tailwind)
├── package.json          # React 19, Vite 8, Tailwind 4, Axios
├── vite.config.ts
├── Dockerfile            # Multi-stage build + Nginx
└── src/
    ├── App.tsx           # Router (8 rutas privadas + login)
    ├── main.tsx          # Entry point
    ├── index.css         # Tema dark global + cyber-card
    ├── api/client.ts     # ApiClient (singleton, axios, 32 métodos)
    ├── hooks/            # useAuth, useOverview, useTopApps, useActivityStream
    ├── pages/            # 10 páginas (Login, Dashboard, Activity, Inventory, USB, Alerts, Security, DeviceDetail, Metrics, Heatmaps)
    ├── components/       # AppShell, Sidebar, OverviewCard, TopAppsTable, ActivityFeed, LiveActivityTable, MetricCard, SimpleLineChart, SimpleBarChart
    └── types/index.ts    # Interfaces del dominio
```

---

## 5. Endpoints API (Server)

### Funcionales (6 endpoints)

| Método | Ruta | Propósito |
|--------|------|-----------|
| GET | `/wifi` | Listar eventos WiFi |
| GET | `/wifi/:device_id` | Eventos WiFi por dispositivo |
| GET | `/agent/policy` | Config remota para agentes |
| GET | `/agent/commands` | Comandos pendientes (placeholder) |
| POST | `/agent/commands/{id}/ack` | Ack de comando |
| GET | `/agent/osquery-policy` | Política osquery |

### No implementados (routers vacíos)

| Ruta | Estado |
|------|--------|
| `/devices/*` | Router vacío |
| `/logs/*` | Router vacío |
| `/inventory/*` | Router vacío |
| `/usb/*` | Router vacío |
| `/security/*` | Router vacío |
| `/heatmaps/*` | Router vacío |

---

## 6. Eventos RabbitMQ

| Queue | Routing Key | Handler | Frecuencia |
|-------|-------------|---------|------------|
| `activity_queue` | `monitoring.activity` | `handle_activity_event` | c/2s |
| `heartbeat_queue` | `monitoring.heartbeat` | `handle_heartbeat_event` | c/60s |
| `inventory_queue` | `monitoring.inventory` | `handle_inventory_event` | 30d |
| `usb_queue` | `monitoring.usb` | `handle_usb_event` | c/60s |
| `wifi_queue` | `monitoring.wifi` | `handle_wifi_event` | c/120s |
| `running_apps_queue` | `monitoring.running_apps` | `handle_running_apps_event` | c/60s |
| `security_queue` | `monitoring.security` | `handle_security_event` | on event |

---

## 7. Base de Datos (PostgreSQL)

13 tablas:

| Tabla | Propósito |
|-------|-----------|
| `devices` | Registro de dispositivos |
| `activity_logs` | Logs de actividad (ventana activa) |
| `running_apps_current` | Snapshot de apps abiertas |
| `inventory` | Inventario de software instalado |
| `usb_events` | Eventos USB (conectar/desconectar) |
| `wifi_events` | Cambios de estado WiFi |
| `input_activity_metrics` | Métricas de entrada (activo/idle por minuto) |
| `node_resource_metrics` | CPU/memoria |
| `processed_events` | Deduplicación de eventos (idempotencia) |
| `audit_events` | Trail de auditoría |
| `security_alerts` | Alertas de seguridad |
| `process_termination_attempts` | Intentos de matar el proceso |
| `security_events` | Eventos osquery + MITRE ATT&CK |
