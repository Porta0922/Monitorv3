# PostgreSQL & Infrastructure Setup Guide
*Actualizado: 28 de Abril, 2026*

## Option 1: Docker Compose (Recommended for Development)

### Prerequisites
- Docker Desktop installed and running
- Docker Compose (included with Docker Desktop)

### Start Services
```bash
docker-compose up -d
```

This starts:
- **PostgreSQL 15** + TimescaleDB on port 5432
- **RabbitMQ 3.12** on ports 5672 (AMQP) and 15672 (Management)
- **Redis 7** on port 6379 (optional, for caching)

### Verify Services
```bash
# Check container status
docker-compose ps

# View logs
docker-compose logs -f

# Stop services
docker-compose down

# Reset and start fresh
docker-compose down -v
docker-compose up -d
```

## Option 2: Manual PostgreSQL Installation

### Windows (using PostgreSQL installer)
1. Download from https://www.postgresql.org/download/windows/
2. Install with default settings (password: `postgres`)
3. Ensure server runs on port 5432

### Windows (using Chocolatey)
```powershell
choco install postgresql
```

### Ubuntu/Debian
```bash
sudo apt-get install postgresql postgresql-contrib
```

### macOS (using Homebrew)
```bash
brew install postgresql
brew services start postgresql
```

### Create Database & User
```bash
# As postgres user
sudo -u postgres psql

# In psql:
CREATE USER monitor_user WITH PASSWORD 'monitor_password';
CREATE DATABASE activity_monitor OWNER monitor_user;

# Grant privileges
GRANT ALL PRIVILEGES ON DATABASE activity_monitor TO monitor_user;
\q

# Verify connection
psql -U monitor_user -d activity_monitor -h localhost
```

## Option 3: Cloud-Hosted PostgreSQL

### PostgreSQL Cloud Providers
- **AWS RDS**: https://aws.amazon.com/rds/postgresql/
- **Azure Database**: https://azure.microsoft.com/services/postgresql/
- **Heroku Postgres**: https://www.heroku.com/postgres
- **DigitalOcean**: https://www.digitalocean.com/products/managed-databases-postgresql
- **Render.com**: https://render.com

### Connection String Format
```
postgresql://monitor_user:monitor_password@host:5432/activity_monitor
```

## TimescaleDB Extension Setup

After creating the database:

```bash
# Connect to database
psql -U monitor_user -d activity_monitor

# Install TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

# Verify installation
\dx

# You should see timescaledb listed
```

## RabbitMQ Setup

### Docker (Recommended)
```bash
docker run -d \
  --name activity-monitor-rabbitmq \
  -p 5672:5672 \
  -p 15672:15672 \
  -e RABBITMQ_DEFAULT_USER=guest \
  -e RABBITMQ_DEFAULT_PASS=guest \
  rabbitmq:3.12-management-alpine
```

### Manual Installation

**Ubuntu/Debian:**
```bash
sudo apt-get install rabbitmq-server
sudo systemctl enable rabbitmq-server
sudo systemctl start rabbitmq-server
```

**macOS:**
```bash
brew install rabbitmq
brew services start rabbitmq
```

**Windows:**
Download from https://www.rabbitmq.com/install-windows.html

### Verify RabbitMQ
- Management UI: http://localhost:15672
- Default credentials: guest / guest

## Database Migrations

After PostgreSQL and TimescaleDB are ready:

```bash
# Apply migrations
psql -U monitor_user -d activity_monitor < migrations/001_init_schema.sql

# Verify schema created
psql -U monitor_user -d activity_monitor -c "\dt"
```

## Environment Configuration

Create `.env` file with your database connection:

```env
DATABASE_URL=postgresql://monitor_user:monitor_password@localhost:5432/activity_monitor
RABBITMQ_URL=amqp://guest:guest@localhost:5672/
```

## Troubleshooting

### Can't connect to PostgreSQL
```bash
# Test connection
psql -U monitor_user -d activity_monitor -h localhost

# Check if server is running
sudo systemctl status postgresql  # Linux
brew services list  # macOS
tasklist /FI "IMAGENAME eq postgres.exe"  # Windows
```

### TimescaleDB extension not found
```bash
# Reinstall PostgreSQL with TimescaleDB
# Or manually install from: https://docs.timescale.com/install/

# Check installed extensions
psql -U monitor_user -d activity_monitor -c "\dx"
```

### RabbitMQ connection refused
```bash
# Check if running
curl localhost:15672  # Should respond

# View logs
docker logs activity-monitor-rabbitmq
# or
tail -f /var/log/rabbitmq/*.log  # Linux
```

## Performance Tuning

### PostgreSQL Configuration
Edit `postgresql.conf`:
```
shared_buffers = 256MB              # 1/4 of RAM
effective_cache_size = 1GB          # 1/2 to 3/4 of RAM
work_mem = 64MB                     # shared_buffers / max_connections
maintenance_work_mem = 64MB
```

### TimescaleDB Configuration
```sql
-- Enable compression for older chunks
ALTER TABLE activity_logs SET (
    timescaledb.compress,
    timescaledb.compress_orderby = 'timestamp DESC'
);
SELECT add_compression_policy('activity_logs', INTERVAL '30 days');
```

## Backup & Recovery

### PostgreSQL Backup
```bash
# Full backup
pg_dump -U monitor_user -d activity_monitor > backup.sql

# Custom format (compressed)
pg_dump -U monitor_user -d activity_monitor -Fc > backup.dump

# Restore
psql -U monitor_user -d activity_monitor < backup.sql
```

### Continuous Archiving (Production)
```sql
-- Enable WAL archiving in postgresql.conf
archive_mode = on
archive_command = 'test ! -f /archive/%f && cp %p /archive/%f'
```

## Health Checks

```bash
# PostgreSQL
psql -U monitor_user -d activity_monitor -c "SELECT version();"

# TimescaleDB
psql -U monitor_user -d activity_monitor -c "\dx"

# RabbitMQ
curl -u guest:guest http://localhost:15672/api/overview

# Connection pool
psql -U monitor_user -d activity_monitor -c "SELECT datname, count(*) FROM pg_stat_activity GROUP BY datname;"
```
