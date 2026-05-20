# Guía de Instalación del Agente Multiplataforma (v3.3.3)
### ActivityMonitor Enterprise

Esta carpeta contiene todos los recursos necesarios para desplegar e instalar de forma interactiva o silenciosa (desatendida) el **Agente de Monitoreo** en sistemas operativos **Windows, Linux y macOS**. 

---

## 🔍 Resolución del Misterio: ¿Por qué antes pedía la carpeta `server`?

En versiones previas, al intentar compilar e instalar el agente en Linux o macOS desde un USB que solo incluía la carpeta `agent`, el compilador arrojaba un error solicitando la carpeta `server`.

**La causa técnica:**
El proyecto está estructurado como un **Workspace de Rust** en su raíz corporativa. El archivo raíz `Cargo.toml` define que los miembros del workspace son `["agent", "server"]`. 
Los scripts de instalación originales intentaban verificar si existía un `Cargo.toml` subiendo directorios, y al encontrar el `Cargo.toml` raíz en el USB, Cargo intentaba compilar en "modo workspace". Para indexar el workspace, Cargo necesita validar y leer los archivos de configuración de todos sus miembros de forma obligatoria. Si la carpeta `server/` no estaba presente, la compilación fallaba por falta de este miembro.

**La Solución Implementada:**
Hemos rediseñado y simplificado los scripts de instalación en esta carpeta. Ahora, los instaladores fuerzan la compilación de forma **100% autónoma (standalone)** directamente dentro de la carpeta `agent/` sin escalar al directorio raíz. **Ya no se requiere en absoluto la carpeta `server/`**, reduciendo a menos de la mitad el peso del USB de despliegue.

---

## 📂 Estructura del USB de Instalación

Para preparar tu USB de despliegue masivo, copia esta carpeta `Instaladores` al USB. Su estructura interna es la siguiente:

```
[USB / Carpeta Instaladores]
│
├── README.md                  <-- Este manual de instrucciones
├── agent/                     <-- Código fuente del agente (con Cargo.toml, src/, etc.)
│
├── Windows/                   <-- Instaladores para Windows
│   ├── install-windows.bat    <-- Instalador interactivo completo (sin falsos positivos)
│   └── install-windows-silent.bat <-- Instalador desatendido / AnyDesk en 1 segundo
│
├── Linux/                     <-- Instaladores para Linux
│   ├── install-linux.sh       <-- Instalador interactivo completo
│   └── install-linux-silent.sh <-- Instalador desatendido / SSH en 1 comando
│
└── macOS/                     <-- Instaladores para macOS
    ├── install-macos.sh       <-- Instalador interactivo completo
    └── install-macos-silent.sh <-- Instalador desatendido / AnyDesk en 1 comando
```

---

## 🔑 Requisitos y Permisos Especiales (¡Muy Importante!)

Tanto Linux como macOS poseen políticas de seguridad restrictivas para la telemetría interactiva del usuario final (rastreo de ventanas, actividad de teclado y ratón). Sigue estrictamente estas instrucciones para asegurar que el agente funcione correctamente:

### 🐧 1. En sistemas Linux

#### A. Permisos de Ejecución del Script
Antes de iniciar, debes otorgar permisos de ejecución al instalador en la terminal:
```bash
chmod +x install-linux.sh install-linux-silent.sh
```

#### B. Privilegios de Administrador (Sudo)
El instalador debe ejecutarse obligatoriamente con `sudo` para poder registrar el servicio de sistema (`systemd`), crear directorios del sistema y aplicar reglas de dispositivos:
```bash
sudo ./install-linux.sh
```

#### C. Permisos de Acceso al Hardware de Entrada (Mouse/Teclado)
Para que el agente de usuario capture la inactividad (idle) y las pulsaciones, requiere acceso a los archivos de dispositivos de entrada en `/dev/input/`.
1. **Reglas Udev**: El instalador creará automáticamente una regla udev en `/etc/udev/rules.d/99-input.rules` para dar permisos de lectura al grupo `input`.
2. **Grupos del Usuario**: El instalador agregará automáticamente al usuario actual de la sesión gráfica a los grupos `input` y `netdev`.
3. **⚠️ REQUISITO CRÍTICO**: Tras finalizar la instalación, **debes cerrar la sesión de usuario y volver a iniciarla** (o reiniciar el equipo) para que se apliquen los nuevos permisos de grupos de hardware. De lo contrario, no capturará actividad.
4. **Wayland vs X11**: Si el sistema utiliza una sesión gráfica *Wayland*, la captura global de teclas puede bloquearse por seguridad. Se recomienda configurar la pantalla de inicio de sesión de Linux en **X11 / Xorg** si notas falta de telemetría de eventos.

---

### 🍏 2. En sistemas macOS

#### A. Permisos de Ejecución del Script
Otorga permisos de ejecución a los instaladores antes de correrlos en la Terminal:
```bash
chmod +x install-macos.sh install-macos-silent.sh
```

#### B. Privilegios de Administrador (Sudo)
Ejecuta el script con `sudo` para poder registrar el demonio del sistema (`LaunchDaemon`) y el agente de sesión (`LaunchAgent`):
```bash
sudo ./install-macos.sh
```

#### C. Permisos de Privacidad TCC de Apple (⚠️ OBLIGATORIO)
macOS posee un estricto subsistema de privacidad llamado TCC (Transparencia, Consentimiento y Control). Por seguridad, bloquea de forma predeterminada cualquier software que intente monitorear la actividad del teclado, mouse o los títulos de las ventanas en segundo plano.

Debes otorgar manualmente los siguientes permisos en los paneles de configuración de la Mac (el instalador interactivo te ofrecerá abrir estos paneles automáticamente al terminar):

1. **Accesibilidad (Accessibility)**:
   * **Qué hace**: Permite al rastreador de inactividad capturar eventos globales de teclado y mouse.
   * **Cómo darlo**: Ve a `Configuración del Sistema > Privacidad y Seguridad > Accesibilidad`.
   * **Acción**: Haz clic en el botón `+`, ingresa tu contraseña de administrador, y agrega el ejecutable del agente en:
     `/Library/Application Support/ActivityMonitor/Bin/activity-monitor-agent`
   * *(Si estás realizando pruebas en la terminal ejecutando el script manualmente, también debes activar la casilla para tu aplicación de **Terminal** o **iTerm** en esta misma lista)*.

2. **Grabación de Pantalla (Screen Recording)**:
   * **Qué hace**: Permite al capturador de ventanas activas leer el nombre del proceso y el título exacto de la ventana en foco (ej. saber si está en "Gmail" o "Excel"). **El agente no toma fotos ni graba video**, solo lee los metadatos de las ventanas activas que macOS agrupa bajo esta política de seguridad.
   * **Cómo darlo**: Ve a `Configuración del Sistema > Privacidad y Seguridad > Grabación de Pantalla`.
   * **Acción**: Agrega el ejecutable del agente (`activity-monitor-agent`) a la lista y asegúrate de que su interruptor esté activado.

3. **⚠️ REQUISITO CRÍTICO**: Si el agente ya estaba abierto al momento de otorgar los permisos, **debes cerrar la sesión de usuario y volver a iniciarla** (o reiniciar la Mac) para que los permisos TCC de Apple surtan efecto y se registre la telemetría gráfica.

---

## ⚡ Instalación Rápida (AnyDesk / Remoto)

Si estás instalando de forma remota por AnyDesk o SSH y ya tienes preparado el archivo `.env` configurado dentro de la carpeta del sistema operativo en el USB, simplemente haz:

* **En Windows (CMD como Admin)**:
  ```cmd
  install-windows-silent.bat
  ```
* **En Linux (Terminal)**:
  ```bash
  sudo ./install-linux-silent.sh
  ```
* **En macOS (Terminal)**:
  ```bash
  sudo ./install-macos-silent.sh
  ```

El proceso se ejecutará de fondo sin hacer preguntas y el agente comenzará a reportar telemetría instantáneamente.
