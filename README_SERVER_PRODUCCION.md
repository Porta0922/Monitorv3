# README - Instalacion del Server en Produccion
*Actualizado: 28 de Abril, 2026*

Guia operativa para desplegar ActivityMonitor Server en produccion y entregarla a otra persona del equipo.

## 1) Objetivo

Levantar en produccion:
- PostgreSQL (TimescaleDB)
- RabbitMQ
- ActivityMonitor Server

Con validaciones de conectividad y correccion del problema de esquema mas comun:
- foreign key inventory_device_id_fkey (VARCHAR vs UUID)

---

## 2) Requisitos previos

En el servidor de produccion:
- Docker y Docker Compose instalados
- Git instalado
- Puertos abiertos:
  - 3000 (API server)
  - 5432 (PostgreSQL, solo si se necesita acceso externo)
  - 5672 y 15672 (RabbitMQ)
- Acceso al repo

Verificar:

```powershell
docker --version
docker compose version
git --version
```

---

## 3) Clonar proyecto

```powershell
git clone <URL_DEL_REPO>
cd ActivityMonitor-Enterprise-v3
```

---

## 4) Levantar infraestructura base (PostgreSQL + RabbitMQ)

Este proyecto ya trae configuracion en docker-compose.yml:
- POSTGRES_DB=activity_monitor
- POSTGRES_USER=monitor_user
- POSTGRES_PASSWORD=monitor_password

Comando:

```powershell
docker compose up -d postgres rabbitmq
```

Validar estado:

```powershell
docker compose ps
docker compose logs -f postgres
```

Probar login a PostgreSQL:

```powershell
docker compose exec postgres psql -U monitor_user -d activity_monitor -c "SELECT current_user, current_database();"
```

---

## 5) Configurar variables del server (.env)

El server carga variables desde archivo .env (dotenv).

### 5.1 Copiar plantilla

```powershell
copy server\.env.example server\.env
```

### 5.2 Editar server/.env

Campos minimos obligatorios:

- SERVER_HOST=0.0.0.0
- SERVER_PORT=3000
- JWT_SECRET=<SECRETO_LARGO_Y_UNICO>
- AGENT_AUTH_TOKEN=<TOKEN_LARGO_Y_UNICO>
- DATABASE_URL=<URL_POSTGRES>
- RABBITMQ_URL=<URL_RABBIT>

### 5.3 DATABASE_URL segun escenario

Escenario A: server corre fuera de Docker (en host)

```env
DATABASE_URL=postgresql://monitor_user:monitor_password@127.0.0.1:5432/activity_monitor
RABBITMQ_URL=amqp://guest:guest@127.0.0.1:5672/
```

Escenario B: server corre dentro de docker-compose (misma red)

```env
DATABASE_URL=postgresql://monitor_user:monitor_password@postgres:5432/activity_monitor
RABBITMQ_URL=amqp://guest:guest@rabbitmq:5672/
```

Nota: usar postgres como host solo funciona si el proceso server corre dentro de la red de Docker Compose.

---

## 6) Iniciar server

Desde la raiz del repo:

```powershell
cargo run -p activity-monitor-server
```

Si se usa binario release:

```powershell
cargo build --release -p activity-monitor-server
.\target\release\activity-monitor-server.exe
```

---

## 7) Validacion funcional

Checklist rapido:
- El server arranca sin error de conexion a PostgreSQL
- El server muestra mensaje de escucha en SERVER_HOST:SERVER_PORT
- RabbitMQ consumer inicia sin reintentos infinitos
- Endpoint de salud responde

Prueba de salud (ajustar puerto si es necesario):

```powershell
Invoke-WebRequest http://127.0.0.1:3000/health
```

---

## 8) Error conocido en produccion y solucion

### Error

foreign key constraint inventory_device_id_fkey cannot be implemented
Detail: Key columns device_id are incompatible types: character varying and uuid.

### Causa

Tabla devices con device_id UUID, pero tabla inventory o activity_logs con device_id VARCHAR en una instalacion vieja.

### Solucion SQL (sin borrar datos)

Ejecutar en PostgreSQL:

```powershell
docker compose exec postgres psql -U monitor_user -d activity_monitor -c "ALTER TABLE inventory DROP CONSTRAINT IF EXISTS inventory_device_id_fkey; ALTER TABLE inventory ALTER COLUMN device_id TYPE UUID USING device_id::uuid; ALTER TABLE inventory ADD CONSTRAINT inventory_device_id_fkey FOREIGN KEY (device_id) REFERENCES devices(device_id);"
```

Y para activity_logs:

```powershell
docker compose exec postgres psql -U monitor_user -d activity_monitor -c "ALTER TABLE activity_logs DROP CONSTRAINT IF EXISTS activity_logs_device_id_fkey; ALTER TABLE activity_logs ALTER COLUMN device_id TYPE UUID USING device_id::uuid; ALTER TABLE activity_logs ADD CONSTRAINT activity_logs_device_id_fkey FOREIGN KEY (device_id) REFERENCES devices(device_id);"
```

Si falla la conversion USING device_id::uuid, hay registros con device_id invalido. En ese caso, limpiar registros invalidos antes de reconvertir.

---

## 9) Problemas frecuentes de credenciales

Si en local funciona y en produccion no:

1. Confirmar que DATABASE_URL del server coincide con POSTGRES_USER/POSTGRES_PASSWORD/POSTGRES_DB de docker-compose.yml.
2. Confirmar host correcto:
- 127.0.0.1 si server corre en host
- postgres si server corre en contenedor
3. Validar acceso real con psql dentro del contenedor postgres.
4. Ver logs de postgres y server en paralelo.

Comandos utiles:

```powershell
docker compose logs -f postgres
docker compose logs -f rabbitmq
```

---

## 10) Error TimescaleDB al iniciar migraciones

### Error

cannot create a unique index without the column "timestamp" (used in partitioning)

### Causa

Una tabla que se convierte en hypertable tenia PRIMARY KEY solo en id.
En TimescaleDB, toda clave unica/primaria debe incluir la columna de particion (timestamp).

### Estado en este repo

La migracion [migrations/002_input_heatmaps_and_alerts.sql](migrations/002_input_heatmaps_and_alerts.sql) y [migrations/003_security_events.sql](migrations/003_security_events.sql) ya fueron corregidas para usar PK compuesta (timestamp, id) en:
- input_activity_heatmaps
- security_alerts
- process_termination_attempts

### Que hacer en un despliegue fallido

Si el error ocurre durante initdb, la BD queda parcialmente inicializada.
La forma mas limpia es recrear volumen de Postgres y relanzar:

```powershell
docker compose down
docker volume ls | findstr postgres_data
# reemplaza <NOMBRE_VOLUMEN> con el que te aparezca en el comando anterior
docker volume rm <NOMBRE_VOLUMEN>
docker compose up -d postgres rabbitmq
docker compose logs -f postgres
```

---

## 11) Hardening minimo recomendado

Antes de pasar a productivo real:
- Cambiar JWT_SECRET por uno fuerte
- Cambiar AGENT_AUTH_TOKEN por uno fuerte
- Cambiar credenciales default de PostgreSQL
- Cambiar credenciales default de RabbitMQ
- Restringir exposicion de puertos en firewall
- Agregar backup programado de volumen postgres_data

---

## 12) Procedimiento de handoff

Para entregar a otro responsable:

1. Entregar este archivo
2. Entregar valores finales de server/.env por canal seguro
3. Ejecutar juntos el checklist de validacion
4. Dejar evidencia (captura/log) de:
- docker compose ps
- prueba de health
- login psql exitoso

---

## 13) Comandos resumen (copiar y ejecutar)

```powershell
# 1) Infra
cd ActivityMonitor-Enterprise-v3
docker compose up -d postgres rabbitmq

# 2) Validar DB
docker compose exec postgres psql -U monitor_user -d activity_monitor -c "SELECT current_user, current_database();"

# 3) Configurar .env
copy server\.env.example server\.env
# editar server/.env

# 4) Ejecutar server
cargo run -p activity-monitor-server
```
