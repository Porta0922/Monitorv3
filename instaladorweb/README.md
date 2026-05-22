# 📦 Instalador Web - Monitor v3

Automatización completa para compilar, empaquetar y desplegar **Monitor v3** en múltiples plataformas (Windows, Linux, macOS).

## 🎯 Características

- ✅ **Compilación Automatizada**: Windows (.exe), Linux y macOS (Intel + ARM64)
- ✅ **GitHub Releases**: Carga automática de binarios a releases
- ✅ **Instalador Web**: Interfaz amigable para generar comandos de instalación
- ✅ **Deployment Automático**: Scripts que descargan e instalan en máquinas
- ✅ **Configuración Automática**: Servicios systemd/launchctl/Windows creados automáticamente
- ✅ **Token Seguro**: Acceso de solo lectura a GitHub

---

## 🚀 Inicio Rápido

### 1️⃣ Usar el Instalador Web (Recomendado)

Abre en tu navegador:
```
https://Porta0922.github.io/Monitorv3/instaladorweb/install.html
```

O copia este enlace:
```
https://raw.githubusercontent.com/Porta0922/Monitorv3/main/instaladorweb/install.html
```

### 2️⃣ Obtener Token de Acceso

1. Ve a [GitHub Settings → Tokens](https://github.com/settings/tokens)
2. Click en **"Generate new token"** → **"Fine-grained tokens"**
3. Configura:
   - **Token name**: `MonitorDeployment`
   - **Expiration**: 90 días
   - **Repository access**: Only select repositories → `Porta0922/Monitorv3`
   - **Permissions**:
     - ✓ Contents: `Read-only`
     - ✓ Releases: `Read-only`
4. Click en **"Generate token"** y copia el token

### 3️⃣ Generar Comando de Instalación

En el instalador web:
1. Selecciona tu plataforma
2. Pega tu token de GitHub
3. Click en **"Generar Comando de Instalación"**
4. Copia el comando que aparece

### 4️⃣ Ejecutar Instalación

**Para Linux/macOS:**
```bash
export MONITOR_DEPLOY_TOKEN="tu_token_aqui"
export MONITOR_REPO="Porta0922/Monitorv3"
curl -sSL https://raw.githubusercontent.com/Porta0922/Monitorv3/main/instaladorweb/deploy-monitor.sh | bash -s linux
```

**Para Windows (PowerShell como Administrador):**
```powershell
$env:MONITOR_DEPLOY_TOKEN="tu_token_aqui"
$env:MONITOR_REPO="Porta0922/Monitorv3"
iex (New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/Porta0922/Monitorv3/main/instaladorweb/Deploy-Monitor.ps1')
```

---

## 📋 Requisitos

### Para Compilación (en tu repositorio)
- ✅ GitHub Actions habilitadas
- ✅ Rust instalado (automático con el workflow)
- ✅ Cargo (incluido con Rust)

### Para Instalación
- ✅ Token de GitHub (solo lectura)
- ✅ Conexión a internet
- ✅ Permisos de administrador (para Linux/macOS necesita `sudo`, Windows requiere ejecutar como Admin)

---

## 🔄 Flujo de Compilación

```
1. Haces push de un tag (git push origin v1.0.0)
                    ↓
2. GitHub Actions dispara el workflow
                    ↓
3. Se compila para:
   ├─ Windows x86_64 (.exe)
   ├─ Linux x86_64
   ├─ macOS x86_64 (Intel)
   └─ macOS ARM64 (Apple Silicon)
                    ↓
4. Se crea una Release con todos los binarios
                    ↓
5. Los scripts de instalación descargan desde la Release
```

### Crear una Release

```bash
# 1. Asegúrate que todo esté committed
git add .
git commit -m "feat: release v1.0.0"

# 2. Crea un tag
git tag -a v1.0.0 -m "Release v1.0.0"

# 3. Push el tag
git push origin v1.0.0

# 4. Ve a https://github.com/Porta0922/Monitorv3/actions
#    y verifica que el workflow se ejecute correctamente
```

---

## 📂 Estructura de Archivos

```
instaladorweb/
├── README.md                    ← Este archivo
├── install.html                 ← Interfaz web del instalador
├── deploy-monitor.sh            ← Script de instalación (Linux/macOS)
└── Deploy-Monitor.ps1           ← Script de instalación (Windows)

.github/workflows/
└── build-release.yml            ← Workflow de compilación automática
```

---

## 🛠️ Scripts de Instalación

### deploy-monitor.sh (Linux/macOS)

**Características:**
- Descarga el binario más reciente
- Crea archivo de configuración (`/etc/monitor/monitor.conf`)
- Configura servicio systemd (Linux)
- Configura launch agent (macOS)
- Logs automáticos

**Uso:**
```bash
export MONITOR_DEPLOY_TOKEN="tu_token"
curl -sSL https://raw.githubusercontent.com/Porta0922/Monitorv3/main/instaladorweb/deploy-monitor.sh | bash -s linux
```

**Directorios creados:**
- **Binario**: `/opt/monitor/monitor`
- **Config**: `/etc/monitor/monitor.conf`
- **Logs**: `/var/log/monitor/`

**Comandos útiles después de instalar:**
```bash
# Ver estado
sudo systemctl status monitor

# Ver logs en tiempo real
sudo journalctl -u monitor -f

# Reiniciar servicio
sudo systemctl restart monitor

# Detener servicio
sudo systemctl stop monitor
```

### Deploy-Monitor.ps1 (Windows)

**Características:**
- Descarga el .exe más reciente
- Crea archivo de configuración
- Registra como servicio Windows
- Inicia automáticamente en el boot
- Control desde Servicios de Windows

**Uso:**
```powershell
$env:MONITOR_DEPLOY_TOKEN="tu_token"
iex (New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/Porta0922/Monitorv3/main/instaladorweb/Deploy-Monitor.ps1')
```

**Directorios creados:**
- **Binario**: `C:\Program Files\Monitor\monitor.exe`
- **Config**: `C:\ProgramData\Monitor\monitor.conf`
- **Logs**: `C:\ProgramData\Monitor\logs\`

**Comandos útiles después de instalar:**
```powershell
# Ver estado
Get-Service -Name Monitor

# Reiniciar servicio
Restart-Service -Name Monitor

# Detener servicio
Stop-Service -Name Monitor

# Ver logs del evento
Get-EventLog -LogName System -Source Monitor | Format-Table
```

---

## 🔐 Seguridad - Token de Acceso

### ¿Por qué solo lectura?

El token está expuesto en el comando de instalación, pero solo tiene permiso para:
- ✅ Leer archivos del repositorio
- ✅ Descargar releases
- ❌ **NO** puede modificar, eliminar o hacer push

### Gestión de Tokens

1. **Crear múltiples tokens** para diferentes propósitos
2. **Establecer expiración** (90 días recomendado)
3. **Revocar tokens comprometidos** inmediatamente
4. **Rotar tokens** periódicamente

**Revocar un token:**
1. Ve a [GitHub Settings → Tokens](https://github.com/settings/tokens)
2. Encuentra el token
3. Click en Delete

---

## 🐛 Solución de Problemas

### Error: "Asset no encontrado"
```
❌ Asset 'monitor-linux-x86_64' no encontrado
```
**Solución**: 
- Verifica que hayas creado un tag y que el workflow se ejecutó
- Ve a [Releases](https://github.com/Porta0922/Monitorv3/releases)
- Confirma que el binario está ahí

### Error: "Límite de API alcanzado"
```
❌ Límite de API de GitHub alcanzado
```
**Solución**:
- Intenta en unos minutos
- Los límites son por hora (60 sin autenticación, 5000 con token)

### El servicio no inicia

**Linux:**
```bash
# Ver detalles del error
sudo systemctl status monitor
sudo journalctl -u monitor -n 50
```

**Windows:**
```powershell
# Ver detalles del error
Get-EventLog -LogName System -Source Monitor | Select-Object -First 5
```

### Actualizar a una nueva versión

Simplemente ejecuta el script de instalación nuevamente:
```bash
bash <(curl -sSL ...) linux
```

El script detectará la versión existente y la actualizará.

---

## 📊 Configuración de Monitor

Archivo: `/etc/monitor/monitor.conf` (Linux/macOS) o `C:\ProgramData\Monitor\monitor.conf` (Windows)

```ini
# Puerto para el servidor web
PORT=8080

# Nivel de logging (debug, info, warn, error)
LOG_LEVEL=info

# Directorio de logs
LOG_DIR=/var/log/monitor

# Intervalo de monitoreo en segundos
CHECK_INTERVAL=60

# Dirección de escucha
BIND_ADDRESS=0.0.0.0
```

**Cambiar configuración:**
1. Edita el archivo de configuración
2. Reinicia el servicio:
   - Linux: `sudo systemctl restart monitor`
   - macOS: `launchctl unload ~/Library/LaunchAgents/com.monitor.plist && launchctl load ~/Library/LaunchAgents/com.monitor.plist`
   - Windows: `Restart-Service -Name Monitor`

---

## 📞 Soporte

¿Problemas o preguntas?

1. Revisa los [Issues en GitHub](https://github.com/Porta0922/Monitorv3/issues)
2. Crea un nuevo issue con detalles
3. Incluye:
   - Sistema operativo y versión
   - Output de los scripts
   - Archivo de configuración (sin token)

---

## 📝 Licencia

Ver [LICENSE](../LICENSE) en el repositorio principal

---

## 🔗 Enlaces Útiles

- 📦 [Repositorio](https://github.com/Porta0922/Monitorv3)
- 📥 [Releases](https://github.com/Porta0922/Monitorv3/releases)
- 🔧 [Issues](https://github.com/Porta0922/Monitorv3/issues)
- 👤 [Autor: Porta0922](https://github.com/Porta0922)

---

**Última actualización**: 2026-05-22
