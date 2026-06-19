=======================================================
  ActivityMonitor Agent - USB Installation v3.3.5
=======================================================

COMO USAR:
  1. Copia toda esta carpeta a un USB
  2. Conecta el USB en la maquina destino
  3. Ejecuta INSTALL.BAT como Administrador
     (o haz doble click y acepta la elevacion de permisos)

  Instalacion silenciosa (remota):
    install-silent.bat

QUE INSTALA:
  - Binary:    C:\ProgramData\ActivityMonitor\Bin\activity-monitor-agent.exe
  - Config:    C:\ProgramData\ActivityMonitor\.env
  - Servicio:  ActivityMonitor (Session 0, inicio automatico)
  - Tarea:     ActivityMonitorUserAgent (se ejecuta al iniciar sesion)
  - Logs:      C:\ProgramData\ActivityMonitor\logs\

CONFIGURACION:
  Para cambiar la configuracion, edita agent-config.json antes de instalar.

  Server:     http://localhost:3000
  RabbitMQ:   amqp://guest:guest@localhost:5672/%2f
  AuthToken:  ***********************

DESINSTALAR:
  En la maquina destino, ejecuta como Admin:
    sc stop ActivityMonitor
    sc delete ActivityMonitor
    schtasks /Delete /TN ActivityMonitorUserAgent /F
    rmdir /s /q C:\ProgramData\ActivityMonitor
