# 📊 Dashboard Completo + Backend Implementado - Guía de Validación

**Estado**: ✅ Implementación Completada

---

## 🎯 Resumen de Cambios Implementados

### Backend Endpoints (Rust/Axum)
| Endpoint | Método | Descripción |
|----------|--------|-------------|
| `/api/overview` | GET | Estadísticas diarias (devices_today, active_time, idle_time, idle_pct, keys_today) |
| `/api/top_apps` | GET | Top 6 aplicaciones en últimos 7 días por duración |
| `/api/stream` | GET (SSE) | Stream de actividades en tiempo real (2 seg refresh) |

### Frontend Components (React/TypeScript/TailwindCSS)
| Componente | Archivo | Función |
|-----------|---------|----------|
| `OverviewCard` | `components/OverviewCard.tsx` | Muestra métricas diarias en cards |
| `TopAppsTable` | `components/TopAppsTable.tsx` | Tabla de 6 apps más usadas |
| `ActivityFeed` | `components/ActivityFeed.tsx` | Stream SSE con actividades en vivo |
| `DashboardPage` | `pages/DashboardPage.tsx` | Página principal integrada |

### Custom Hooks (React)
| Hook | Archivo | Propósito |
|------|---------|----------|
| `useOverview` | `hooks/useOverview.ts` | Fetch /api/overview cada 30 seg |
| `useTopApps` | `hooks/useTopApps.ts` | Fetch /api/top_apps cada 60 seg |
| `useActivityStream` | `hooks/useActivityStream.ts` | EventSource con reconexión automática |

### Mejoras del Agente (Rust)
| Módulo | Archivo | Característica |
|--------|---------|-----------------|
| `KeystrokeTracker` | `agent/src/keystroke_tracker.rs` | Rastreo de keystrokes + idle detection |
| Windows Hook | `keystroke_tracker.rs:windows_input_listener` | Low-level keyboard + mouse hooks |
| Idle Detection | `KeystrokeTracker::update_idle_status()` | Deteccion automática si inactivo >5min |

---

## 🚀 Instrucciones para Validar el Sistema Completo

### PASO 1: Compilar Backend y Dashboard
```bash
# En una terminal - Server (Rust)
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3\server
cargo build --release

# En otra terminal - Dashboard (React)
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3\dashboard
npm run build
```

### PASO 2: Iniciar Servicios
```bash
# Terminal 1: Docker (PostgreSQL + RabbitMQ)
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3
docker-compose up -d

# Terminal 2: Server
cd server
cargo run --release
# Esperaado ver: ✅ Connected to PostgreSQL, ✅ Server listening on http://0.0.0.0:3000

# Terminal 3: Agent
cd agent
cargo run --release
# Esperado ver: ✅ Agent connected to RabbitMQ, 📤 Publishing event...

# Terminal 4: Dashboard Dev
cd dashboard
npm run dev
# Esperado ver: ➜ Local: http://localhost:5173/
```

---

## ✅ Checklist de Validación

### Backend Endpoints
- [ ] `GET http://localhost:3000/api/overview` → JSON con estadísticas 
- [ ] `GET http://localhost:3000/api/top_apps` → JSON con 6 apps
- [ ] `GET http://localhost:3000/api/stream` → SSE stream funciona (conectar con curl o navegador)
- [ ] Todos los endpoints retornan datos sin errores 500

### Frontend Dashboard
- [ ] Abir http://localhost:5173 en navegador
- [ ] Login: usuario `admin`, contraseña `password123`
- [ ] **Overview Card**:
  - [ ] Muestra "Devices Today" (número de dispositivos)
  - [ ] Muestra "Active Time" en horas
  - [ ] Muestra "Idle Time" en horas
  - [ ] Muestra "Idle %" con valor correcto
  - [ ] Muestra "Keys Today" (0 por ahora)
  
- [ ] **Top Apps Table**:
  - [ ] Muestra hasta 6 aplicaciones
  - [ ] Cada app tiene duración en horas y segundos
  - [ ] Se actualiza cada 60 segundos
  
- [ ] **Live Activity Stream**:
  - [ ] Indicador verde "Connected" visible
  - [ ] Muestra lista de actividades con app, título, estado (Idle/Active)
  - [ ] Se actualiza cada 2 segundos
  - [ ] Timestamp mostrado es reciente
  
- [ ] **Devices Section**:
  - [ ] Lista dispositivos registrados
  - [ ] Muestra estado online/offline
  - [ ] Muestra MAC address
  - [ ] Last seen actualizado

### Idle Detection (Agente)
- [ ] No mover mouse/keyboard por 5+ minutos
- [ ] En Activity Feed, se debe marcar como "🛌 Idle"
- [ ] Status cambia automáticamente cuando reanudan actividad

---

## 📊 Ejemplo de Respuestas Esperadas

### GET /api/overview
```json
{
  "success": true,
  "data": {
    "devices_today": 1,
    "active_time": 14400,
    "idle_time": 3600,
    "idle_pct": "20.0%",
    "keys_today": 0
  }
}
```

### GET /api/top_apps
```json
{
  "success": true,
  "data": [
    {
      "app_name": "chrome.exe",
      "total_duration_seconds": 72000,
      "total_duration_hours": "20.00"
    },
    ...
  ]
}
```

### GET /api/stream (SSE)
```json
{
  "activities": [
    {
      "device_id": "550e8400-e29b-41d4...",
      "app": "vscode.exe",
      "title": "DashboardPage.tsx - VS Code",
      "is_idle": false,
      "is_live": true,
      "last_seen": "2026-04-02T10:45:30Z"
    }
  ],
  "timestamp": "2026-04-02T10:45:32Z"
}
```

---

## 🔧 Troubleshooting

### Stream SSE no conecta
**Problema**: Activity Feed muestra "Desconectado"
**Solución**: 
1. Verificar que servidor está corriendo: `curl http://localhost:3000/api/health`
2. Check CORS: Dashboard debe estar en http://localhost:5173
3. Abrir DevTools (F12) → Network y revisar logs de conexión

### Top Apps no muestra datos
**Problema**: Tabla de apps vacía
**Solución**:
1. El agente debe estar enviando eventos de activity
2. Esperar 2+ eventos de activity registrados en la BD
3. Revisar logs del servidor para errors

### Overview muestra 0 en todos los campos
**Problema**: Estadísticas en cero
**Solución**:
1. Deve haber datos de activity_logs en BD para ayer y hoy
2. Ejecutar: `curl http://localhost:3000/api/logs` para verificar
3. Si vacío, el agente no está publicando eventos

### Dashboard lento
**Problema**: Interfaz lenta o se congela
**Solución**:
1. Revisar: Activity Feed actualiza cada 2 seg (puede ser heavy)
2. Limitar SSE a max 50 actividades visibles
3. Aumentar intervalo a 5-10 segundos si es necesario

---

## 📈 Próximos Pasos (Fase 4)

- [ ] Implementar WebSocket en lugar de SSE para mejor rendimiento
- [ ] Agregar Alertas en tiempo real
- [ ] Implementar gráficos de uso por hora/día
- [ ] Exportar reportes en PDF
- [ ] Dashboard móvil responsive
- [ ] Autenticación con OAuth2
- [ ] Métricas de seguimiento automático (keystroke count)

---

## 📝 Notas Técnicas

### Tasas de Refresco Configuradas
- **Overview**: 30 segundos (datos no cambian frecuentemente)
- **Top Apps**: 60 segundos (historial semanal, cambios graduales)
- **Activity Stream**: 2 segundos (real-time, sensible a cambios)

### Idle Detection
- Umbral: 5 minutos (300 segundos) sin input
- Detectado a nivel: agent monitoring loop (cada 2 sec)
- Indicador visual: Color gris en Activity Feed

### SSE Reconexión
- Intervalo: 5 segundos después de desconexión
- Auto-recovery: Sí, automático
- Max actividades mostradas: Todas (considerar limitar a 20 en producción)

---

**¡Listo para validar!** 🎉

Una vez completado este checklist, el dashboard está completamente funcional y listo para monitoreo en tiempo real.
