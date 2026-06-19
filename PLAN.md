# Plan de Mejoras — ActivityMonitor Enterprise v3

> Priorizado por impacto y esfuerzo estimado.

---

## 🚨 Prioridad Crítica (Seguridad y Funcionalidad Básica)

### 1. Implementar autenticación en rutas API
- **Archivos**: `server/src/domains/*/routes.rs`, `server/src/auth.rs`
- **Problema**: `AuthManager` (JWT + Argon2id) está implementado pero **nunca se llama**. No hay middleware de auth en ninguna ruta. El dashboard envía `Authorization: Bearer <token>` pero el server lo ignora.
- **Acción**: Crear un middleware Axum que verifique el JWT en rutas protegidas. Implementar endpoint `/auth/login` que use `AuthManager`.

### 2. Implementar endpoints API faltantes (6 routers vacíos)
- **Archivos**: `server/src/domains/{device,activity,inventory,usb,security,keystroke}/routes.rs`
- **Problema**: El dashboard tiene 32+ métodos API que apuntan a endpoints que **no existen**. El servidor devuelve 404 para `/devices`, `/logs`, `/inventory`, `/usb`, `/security`, `/heatmaps`.
- **Acción**: Implementar los endpoints en cada dominio usando `Database` (que ya tiene los métodos CRUD). Los repositorios duplicados (`domains/*/repository.rs`) deberían eliminarse o convertirse en la fuente única.

### 3. Reemplazar `CorsLayer::permissive()`
- **Archivo**: `server/src/api.rs:21`
- **Problema**: CORS abierto a cualquier origen en producción.
- **Acción**: Configurar CORS específico para el origen del dashboard (o mediante env var `CORS_ORIGIN`).

### 4. Eliminar secretos hardcodeados
- **Archivos**: `server/src/main.rs:27`, `server/src/domains/agent/routes.rs`, `agent/src/main.rs:395`, `scripts/build-release.ps1`, `scripts/build-usb.ps1`
- **Problema**: `JWT_SECRET`, `AGENT_AUTH_TOKEN`, `AGENT_OFFLINE_CACHE_KEY` con valores por defecto predecibles.
- **Acción**: Forzar configuración vía env vars sin defaults inseguros; generar automáticamente en primera ejecución.

---

## 🔧 Prioridad Alta (Deuda Técnica y Calidad)

### 5. Unificar `postgres_db.rs` y repositorios por dominio
- **Archivos**: `server/src/postgres_db.rs` (~2071 líneas) + `server/src/domains/*/repository.rs` (6 archivos duplicados)
- **Problema**: ~14 pares de métodos idénticos. Las rutas nuevas deberían llamar al repository, no a `Database`.
- **Acción**: Migrar toda la lógica DB a los repositorios de dominio y hacer que `postgres_db.rs` delegue en ellos, o viceversa. Eliminar la duplicación.

### 6. Integrar WebSocket (`ws.rs`)
- **Archivo**: `server/src/ws.rs` (143 líneas, muerto)
- **Problema**: `WsSubscriber` y `WsMessage` están completamente implementados pero no se usan. No hay endpoint `/ws`, no está en `AppState`, no se importa en `main.rs`.
- **Acción**: Agregar endpoint WebSocket con Axum, conectar `WsSubscriber` a `AppState`, emitir eventos desde el consumidor RabbitMQ a los subscriptores.

### 7. Agregar capa de caché en el dashboard
- **Archivos**: `dashboard/src/pages/*.tsx`, `dashboard/src/hooks/*.ts`
- **Problema**: Cada página refetcha datos al montar. 4+ componentes independientes hacen polling a diferentes intervalos (10s, 15s, 30s, 60s).
- **Acción**: Integrar React Query (TanStack Query) o SWR para cache/deduplicación/refetch inteligente. Unificar polling en un solo loop.

### 8. Cobertura de tests
- **Dashboard**: 0 tests
- **Server**: 6 tests (4 auth + 2 ws)
- **Agent**: Tests mínimos, muchos `#[ignore]`
- **Acción**: Agregar tests unitarios para componentes clave en dashboard y server. Tests de integración para el pipeline RabbitMQ → DB → API.

---

## 📋 Prioridad Media (Mejoras Estructurales)

### 9. Eliminar archivos muertos y duplicados

| Archivo | Acción |
|---------|--------|
| `dashboard/src/App.css` | Eliminar |
| `dashboard/src/components/NavBar.tsx` | Eliminar (reemplazado por Sidebar+AppShell) |
| `dashboard/src/pages/HeatmapsPage.tsx` | Eliminar o registrar en router |
| `server/src/ws.rs` | Integrar o eliminar |
| `scratch/*` (12 archivos) | Archivar o eliminar |
| `domains/{keystroke}/*` | Implementar o eliminar |
| `errors.json` | Verificar si se usa |

### 10. Refactorizar archivos grandes
- `server/src/postgres_db.rs` (2071 líneas) → dividir por dominio/subsistema
- `dashboard/src/pages/DeviceDetailPage.tsx` (829 líneas) → extraer hooks y subcomponentes
- `server/src/rabbitmq_consumer.rs` (804 líneas) → extraer handlers a archivos separados

### 11. Unificar InputTracker y KeystrokeTracker
- **Archivos**: `agent/src/input_tracking.rs`, `agent/src/keystroke_tracker.rs`
- **Problema**: `InputTracker` (heatmap grid) y `KeystrokeTracker` (contadores atómicos) miden cosas solapadas (ratón, teclado). Cada uno hace idle detection por separado.
- **Acción**: Unificar en un solo `InputMonitor` que exponga heatmaps + contadores + idle.

### 12. Tipado estricto en Dashboard
- **Archivos**: `dashboard/src/**/*.tsx`
- **Problema**: Uso extensivo de `any`, `as any`, `as Type[]` casts.
- **Acción**: Activar `strict: true` en tsconfig, tipar todas las respuestas API, eliminar casts.

### 13. Estandarizar manejo de errores
- **Problema**: En `rabbitmq_consumer.rs` algunos errores se loguean y continúan, otros rompen el loop. El dashboard usa `console.error` disperso. No hay Error Boundaries en React.
- **Acción**: Crear sistema consistente de manejo de errores en servidor y dashboard (Error Boundary, toast notifications).

---

## 🧹 Prioridad Baja (Calidad de Código)

### 14. Unificar idioma (español → inglés)
- **Archivos**: `rabbitmq_consumer.rs`, `agent/src/main.rs`, scripts PowerShell, dashboard (múltiples)
- **Problema**: Mensajes mezclan español e inglés (ej: `"Sin titulo"`, `"Copia a USB detectada"` en logging técnico).
- **Acción**: Estandarizar todo el código fuente a inglés. Mantener español solo en UI si es necesario.

### 15. Agregar `// SAFETY:` comments en bloques unsafe
- **Archivos**: `agent/src/keystroke_tracker.rs`, `agent/src/process_protection.rs`, `agent/src/main.rs`
- **Problema**: Múltiples bloques `unsafe` sin justificación documentada.

### 16. Refactorizar paths hardcodeados
- **Problema**: `C:\ProgramData\ActivityMonitor` y `/var/lib/activity-monitor` repetidos en múltiples archivos.
- **Acción**: Definir constantes globales (o env vars) para paths del sistema.

### 17. Reemplazar `println!` por `tracing` en rabbitmq_consumer
- **Archivo**: `server/src/rabbitmq_consumer.rs`
- **Problema**: 20+ `println!` que bypassan el sistema de logging estructurado.

---

## Resumen de Esfuerzo Estimado

| Prioridad | Ítems | Esfuerzo estimado |
|-----------|-------|-------------------|
| 🚨 Crítica | 4 items | ~2-3 semanas |
| 🔧 Alta | 4 items | ~2-3 semanas |
| 📋 Media | 5 items | ~1-2 semanas |
| 🧹 Baja | 4 items | ~3-5 días |
| **Total** | **17 items** | **~6-9 semanas** |
