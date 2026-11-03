# Políticas del servicio y la sesión

Estado 2026-11-02: implementación y comprobaciones focalizadas del bloque 4 aprobadas; no desplegada en el piloto. Ver BLOCK4_CLOSURE.md.

## Flujo

Servidor → servicio: consulta existente cada 300 segundos con credencial del agente.
Servicio → sesión: consulta cifrada de política cada 15 segundos por el relay local.
No se comparten credenciales del servidor con la sesión. Cada respuesta se vincula al
UUID del dispositivo y solo expone configuración de recolectores, no secretos.

El servidor publica una configuración completa con valores predeterminados válidos;
antes enviaba intervalos cero. Los campos numéricos se validan en el agente. Un
snapshot inválido o que no se pueda persistir deja la configuración anterior activa.
No se interpretan valores predeterminados como «ignorar»: se pueden restaurar intervalos
y vaciar listas mediante una configuración posterior.

## Selección y pausa

- enabled_monitors vacío mantiene la compatibilidad: habilita todos los disponibles.
- Un monitor no puede estar en ambas listas ni repetirse; los snapshots nuevos contradictorios se rechazan. La selección de recolectores mantiene la prioridad defensiva de disabled_monitors.
- monitoring_paused suspende los recolectores supervisados; transporte, salud y
  comandos continúan para permitir recuperación y administración.
- session_input hereda la selección de resource_logger para políticas anteriores.

Los wrappers supervisados crean/detienen recolectores al cambiar la política, incluso
si empezaron deshabilitados. Los cambios de intervalo interrumpen la espera del próximo
ciclo. El umbral de inactividad se actualiza en el tracker. Al reactivar entrada/mapas de
sesión se descartan acumuladores transitorios del período deshabilitado.

La escritura de nuevos eventos vuelve a revisar la política. Los eventos ya en cola
no se eliminan. El relay difiere los tipos deshabilitados sin confirmarlos, de modo que
la sesión pueda reintentarlos cuando vuelvan a habilitarse. Si los primeros 256 eventos
son de tipos pausados, los posteriores pueden quedar retenidos por el orden de lectura;
la planificación por tipo queda pendiente. El servicio puede seguir enviando eventos
que ya había aceptado antes de la pausa.

No se promete desmontar todos los hooks nativos ni cancelar inmediatamente procesos
externos heredados. La pausa controla tareas y publicación; la instrumentación de bajo
nivel y los workers no supervisados necesitan completar el ciclo de vida en P2.

## Persistencia

Cada rol conserva effective_policy.json en su carpeta de estado. La sustitución usa
archivo temporal en la misma carpeta, sync y rename/MoveFileExW. En Unix se crea con
0600; en Windows hereda ACL de la carpeta y el paquete debe protegerla.

La sesión exige una política obtenida del servicio antes de su primera recolección.
Después puede arrancar offline con la última configuración validada. Una configuración
persistida corrupta provoca fallo de arranque; no activa valores permisivos silenciosamente.
Los cambios remotos no llegan mientras no haya conexión: la última política sigue vigente.

La configuración persistida prevalece sobre los valores iniciales del entorno. La
revisión config_version es local al proceso, se reinicia con él y no es una versión
global de política. La sincronización compara contenidos, no depende de ese contador.

## Configuración del servidor

Compose y .env.example exponen los intervalos AGENT_CONFIG_* y:

- AGENT_CONFIG_MONITORING_PAUSED=false
- AGENT_CONFIG_ENABLED_MONITORS= (lista separada por comas)
- AGENT_CONFIG_DISABLED_MONITORS= (lista separada por comas)

Cambiar el .env no modifica contenedores existentes: aplicar cambios en despliegue
cuando se retome la validación. No hay editor administrativo ni reglas por grupos aún.
La respuesta completa del servidor es autoritativa y puede reemplazar intervalos locales.

## P4

Validar defaults, restauración de valores, pausa/reanudación, activación tardía,
identidad de respuesta, snapshots inválidos, disco sin permisos/lleno y reemplazo atómico
en Windows/Linux. Ensayar cambio durante escritura, eventos pendientes y cierre de
wrappers sin tareas huérfanas. Ninguna de estas comprobaciones se ejecutó en este bloque.
