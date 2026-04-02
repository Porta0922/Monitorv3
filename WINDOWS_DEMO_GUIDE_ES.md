# ActivityMonitor Enterprise v3 — Guía Completa de Demo en Windows

**Guía Paso a Paso: Desde la Instalación hasta Ver el Dashboard de Monitoreo en Tiempo Real**

Esta guía te enseña cómo ejecutar una **demostración completa y funcional** de ActivityMonitor, desde la configuración inicial hasta visualizar datos en tiempo real en el dashboard.

**⏱️ Tiempo total: ~1 hora (30 minutos de configuración + 30 minutos de pruebas)**

---

## Requisitos Previos

Antes de comenzar, asegúrate de tener:

- **Windows 10/11** (64-bit)
- **Docker Desktop para Windows** (instalado y en ejecución, con contenedores Linux habilitados)
- **Rust 1.70+** instalado
- **Node.js 18+** instalado
- **Git** (opcional, para clonar)
- **Permisos de administrador** (para instalación de servicios)
- **~5 GB de espacio en disco** disponibles
- **Puertos disponibles**: 3000, 5173, 5432, 5672, 15672
  - Si alguno está en uso, cierra los programas: SQL Server, servicios existentes, etc.

### Verificación Rápida

```powershell
# Abre PowerShell como Administrador y ejecuta:
docker --version          # Debe mostrar la versión de Docker
docker ps                 # Debe funcionar sin errores
rustc --version          # Debe mostrar Rust 1.70+
node --version           # Debe mostrar Node.js 18+
git --version            # Opcional pero recomendado
```

**✅ Si todos los comandos funcionan, ¡estás listo! Si no, instala las herramientas faltantes.**

---

## 🎯 RESUMEN RÁPIDO

Lo que harás:
1. Iniciar servicios Docker (PostgreSQL, RabbitMQ)
2. Compilar y ejecutar el servidor Rust
3. Compilar e instalar el agente de Windows
4. Iniciar el dashboard React
5. Conectarte y observar el monitoreo de actividad en tiempo real

**Este es un sistema COMPLETO y funcional en ~1 hora.**

---

## RECORRIDO PASO A PASO

### PASO 1: Abrir Directorio del Proyecto (1 minuto)

```powershell
# Abre PowerShell como Administrador
# Navega a la raíz del proyecto:
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3

# Verifica que estés en el lugar correcto:
dir  # Debe mostrar: agent/, server/, dashboard/, docker-compose.yml
```

---

### PASO 2: Iniciar Servicios Docker (5 minutos)

```powershell
# Inicia todos los servicios backend
docker-compose up -d

# Espera 10 segundos para que los contenedores se inicialicen
Start-Sleep -Seconds 10

# Verifica que todos los contenedores estén en ejecución
docker-compose ps
```

**Salida Esperada**:
```
NAME              STATUS
postgres          Up (healthy)
rabbitmq          Up (healthy)
```

**✅ Los servicios backend ya están en ejecución!**

---

### PASO 3: Compilar el Servidor Rust (15 minutos)

```powershell
# Navega al directorio del servidor
cd server

# Compila el binario en modo release (tarda unos minutos)
cargo build --release

# Espera a que se complete...
```

**Salida Esperada** (al final):
```
Finished release [optimized] target(s) in 45.23s
```

**✅ Servidor compilado exitosamente!**

---

### PASO 4: Ejecutar el Servidor (5 minutos)

```powershell
# Aún en el directorio 'server'
# Ejecuta el servidor en modo release
cargo run --release

# Espera a ver:
# ✅ Connected to PostgreSQL
# ✅ Server listening on http://0.0.0.0:3000
# ✅ RabbitMQ consumer started
```

**⚠️ IMPORTANTE**: Deja esta ventana abierta. El servidor sigue ejecutándose.

**✅ Servidor ejecutándose en puerto 3000**

---

### PASO 5: Compilar e Instalar el Agente (NUEVA VENTANA) (10 minutos)

```powershell
# Abre una NUEVA ventana de PowerShell como Administrador
# Navega a la raíz del proyecto
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3\agent

# Compila el agente
cargo build --release

# Espera a que se complete...
```

**Salida Esperada**:
```
Finished release [optimized] target(s) in X.XXs
```

**✅ Agente compilado!**

---

### PASO 6: Ejecutar el Agente (Aún en la ventana del agente)

```powershell
# Ejecuta el agente
cargo run --release

# Deberías ver logs como:
# ✅ Agent connected to RabbitMQ
# 📤 Publishing event: activity
# ✅ Event published successfully
```

**⚠️ IMPORTANTE**: Deja esta ventana abierta. El agente sigue ejecutándose y enviando datos.

**✅ Agente enviando datos a RabbitMQ**

---

### PASO 7: Compilar y Ejecutar el Dashboard (NUEVA VENTANA) (10 minutos)

```powershell
# Abre una NUEVA ventana de PowerShell (NO necesita ser Administrador)
# Navega a la raíz del proyecto
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3\dashboard

# Instala dependencias
npm install

# Inicia el servidor de desarrollo
npm run dev

# Espera a ver:
# ➜ Local: http://localhost:5173/
```

**✅ Dashboard ejecutándose!**

---

### PASO 8: Acceder al Dashboard en el Navegador (2 minutos)

```
Abre tu navegador en: http://localhost:5173
```

**Lo que deberías ver**:
1. Pantalla de login
2. Usuario: `admin` | Contraseña: `password123`
3. Haz clic en "Iniciar Sesión"

**✅ ¡Has entrado al dashboard!**

---

### PASO 9: Ver los Datos en Tiempo Real (5 minutos)

Una vez logueado, deberías ver:

- **Sección "Dispositivos"**: Tu computadora registrada
- **Sección "Actividad"**: Logs de aplicaciones abiertas
- **Sección "Inventario"**: Listado de aplicaciones instaladas
- **Sección "Heatmaps"**: Gráficos de actividad de teclado/ratón

**Prueba esto para confirmar que funciona:**

1. Abre varias aplicaciones (Notepad, Chrome, etc.)
2. Escribe en ellas
3. Actualiza el dashboard (F5)
4. **Deberías ver la actividad actualizada en tiempo real**

---

## ✅ LISTA DE VERIFICACIÓN DE ÉXITO

Marca cada paso conforme lo completes:

- [ ] Docker: PostgreSQL y RabbitMQ en ejecución
- [ ] Servidor compilado sin errores
- [ ] Servidor ejecutándose en puerto 3000
- [ ] Agente compilado sin errores
- [ ] Agente ejecutándose y enviando eventos
- [ ] Dashboard compilado sin errores
- [ ] Dashboard ejecutándose en puerto 5173
- [ ] Puedo acceder a http://localhost:5173
- [ ] Puedo iniciar sesión (admin/password123)
- [ ] Veo dispositivos registrados
- [ ] Veo logs de actividad
- [ ] Veo inventario de software

**Si todas están marcadas: ¡FELICIDADES! El sistema completo funciona correctamente.**

---

## 🔧 SOLUCIÓN DE PROBLEMAS

### Docker no inicia

**Problema**: `docker-compose up -d` falla

**Solución**:
```powershell
# Verifica que Docker está instalado y en ejecución
docker ps

# Si ves errores, reinicia Docker Desktop
# Y asegúrate de que "Linux containers" está habilitado
```

### Servidor no se conecta a PostgreSQL

**Problema**: `❌ Failed to connect to PostgreSQL`

**Solución**:
```powershell
# Verifica que PostgreSQL está en ejecución
docker-compose ps | grep postgres

# Debe mostrar: postgres ... Up (healthy)

# Si no está en ejecución:
docker-compose up -d postgres
```

### Agente no puede conectarse a RabbitMQ

**Problema**: Error de conexión en el agente

**Solución**:
```powershell
# Verifica que RabbitMQ está en ejecución
docker-compose ps | grep rabbitmq

# Debe mostrar: rabbitmq ... Up (healthy)

# Reinicia RabbitMQ:
docker-compose down rabbitmq
docker-compose up -d rabbitmq
```

### Dashboard no muestra datos

**Problema**: Dashboard abierto pero sin datos

**Solución**:
1. Verifica que el servidor está ejecutándose (ve a: http://localhost:3000/api/devices)
2. Verifica que el agente está ejecutándose (ve a la ventana del agente)
3. Espera 10 segundos y actualiza el dashboard (F5)
4. Si aún no hay datos, revisa los logs del servidor y agente

### Puerto ya en uso

**Problema**: `Address already in use`

**Solución**:
```powershell
# Encuentra qué proceso está usando el puerto 3000
netstat -ano | findstr :3000

# Mata el proceso (reemplaza XXXX con el PID):
taskkill /PID XXXX /F

# O cambia el puerto en el servidor:
# Edita .env y añade: SERVER_PORT=3001
```

---

## 📝 COMANDOS RÁPIDOS DE REFERENCIA

```powershell
# Iniciar servicios Docker
docker-compose up -d

# Ver logs de un servicio
docker-compose logs postgres  # o rabbitmq

# Detener servicios Docker
docker-compose down

# Compilar server
cd server && cargo build --release

# Ejecutar server
cd server && cargo run --release

# Compilar agente
cd agent && cargo build --release

# Ejecutar agente
cd agent && cargo run --release

# Instalar dependencias del dashboard
cd dashboard && npm install

# Ejecutar dashboard
cd dashboard && npm run dev

# Abrir dashboard
start http://localhost:5173

# Revisar RabbitMQ
start http://localhost:15672  # Usuario: guest, Contraseña: guest
```

---

## 🎯 NEXT STEPS (Próximos Pasos)

Ahora que tienes el sistema funcionando:

1. **Personaliza el agente**: Edita `agent/src/main.rs` para cambiar el intervalo de monitoreo
2. **Añade más dispositivos**: Ejecuta el agente en otra computadora
3. **Implementa alertas**: Configura notificaciones en tiempo real
4. **Integra con tu sistema**: Conecta ActivityMonitor a tu infraestructura

---

## 📞 SOPORTE

Si encuentras problemas:

1. **Revisa los logs**:
   - Servidor: mira la terminal del servidor
   - Agente: mira la terminal del agente
   - Dashboard: abre Developer Tools (F12) → Console

2. **Verifica la conectividad**:
   ```powershell
   curl http://localhost:3000/api/devices
   ```

3. **Reinicia todo**:
   ```powershell
   docker-compose down
   docker-compose up -d
   # Luego reinicia servidor, agente y dashboard
   ```

---

**¡Bienvenido a ActivityMonitor! 🎉**
