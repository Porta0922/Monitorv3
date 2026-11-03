# Renovación UX del dashboard

Implementación en código del 2026-10-24. No se ejecutaron compilaciones, pruebas,
servidores de desarrollo ni revisión visual: toda validación está en P4 por pedido
del usuario. El stack Docker existente no se actualizó.

## Dirección visual

Monitor adopta superficies claras, navegación lateral verde grisácea, tarjetas
blancas, tipografía del sistema y verde reservado para acciones/estados positivos.
La nueva vista de flota prioriza conexión, última señal y alertas antes que contadores
de teclado. Picos históricos y mediciones actuales se nombran por separado.

La estructura compartida incluye menú móvil nativo, encabezado, acciones de página,
contenido desplazable y navegación agrupada. No depende de Google Fonts ni imágenes
remotas. Los SVG de interfaz y marca están incluidos en el repositorio.

## Flujos

- Flota: búsqueda por nombre/hostname/MAC/ID, conexión, orden y página en la URL.
  Paginación local de 20 filas. Los filtros sobreviven a volver atrás o compartir URL.
- Cada equipo tiene un enlace de detalle y un diálogo de administración. Cambiar nombre
  muestra resultado; desinstalar requiere escribir DESINSTALAR y solo confirma encolado.
- Alertas: abiertas/resueltas, severidad y búsqueda. La resolución solicita confirmación
  y muestra fallos sin eliminar el registro de la lista como si hubiera sido exitoso.
- Métricas: resumen global y selección obligatoria de equipo para detalle horario;
  actividad en minutos, históricos diarios en UTC y mantenimiento en sección aparte.
- Inventario/USB/WiFi: patrón de tabla compartido con búsqueda, 25 filas por página,
  estados vacíos/carga/error y límites de consulta explícitos.
- Detalle: se conservan actividad, aplicaciones, conexiones y resúmenes; se adapta la
  distribución y se ignoran respuestas de una selección anterior.

## Semántica de datos

La flota consulta cada 30 segundos con la pestaña visible. Conserva datos durante el
refresco y avisa si falla alguna fuente. No asigna cero a consultas fallidas de alertas
o recursos. La fecha de consulta no equivale a la fecha de última señal del agente.

Señal atrasada usa el campo stale de la API. No significa diagnóstico de agente
degradado. El consumo mostrado en la portada son máximos diarios, hasta 20 equipos
según el resultado de la API. Las conexiones USB/WiFi describen eventos históricos.

La API horaria exige UUID; la antigua consulta con cadena vacía fue sustituida por
selección de equipo. El cliente rechaza success=false en resumen/horas. El detalle
horario usa zona local; resumen diario usa UTC y limita el día final a hoy UTC.

## Pendiente

Validar todo en P4, especialmente TypeScript, Tailwind 4, responsive, contraste y
teclado. La inclusión de estilos no constituye una comprobación visual.
Grupos, columnas configurables, diagnóstico de salud, onboarding y gestión de versiones
siguen pendientes. La API aún no comunica disponibilidad de todos los recolectores:
los contadores heredados no permiten deducir soporte a partir de un valor cero.
La paginación de colecciones es local; no amplía los límites impuestos por la API.

## UX del detalle del equipo — 2026-10-28

- Entrada predeterminada en Resumen: identidad compacta, cuatro indicadores de recursos/salud, última actividad reportada, cinco aplicaciones con más uso y accesos directos. Retirados los contadores heredados engañosos del encabezado.
- Secciones independientes: Resumen, Actividad, Aplicaciones, Recursos, USB, WiFi y Diagnóstico. El diagnóstico y las sesiones ya no aparecen delante de la actividad.
- Actividad agrupada por aplicación, búsqueda por aplicación/título, filtro por hora, ocho aplicaciones por página y ventanas desplegables (diez iniciales, cargar más). Se conservan tiempos por persona y se distinguen del tiempo encendido.
- Recursos: métricas actuales separadas de historia; almacenamiento/red desplegables. Fechas de actividad inicializadas con el calendario local.
- Verificación: build TypeScript/Vite y contrato de agrupación aprobados. Navegador con equipo real: resumen carga, actividad limita a ocho filas y búsqueda Excel reduce a una aplicación.
- Pendiente general: consolidación de contadores de entrada por sesión, ciclo de reparación/desinstalación y recuperación, escenarios Linux/multisesión.
