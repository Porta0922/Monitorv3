# Dashboard: UX y operación diaria

Actualizado: 2026-11-03. Implementación del bloque 8 completada; aceptación con usuarios y certificación asistiva completa en bloque 9. No implica aprobación de producción.

## Recorridos e información

- Equipos: búsqueda, área, estado, orden y página en URL. Abrir un equipo conserva la consulta de origen para volver al listado.
- Detalle: Resumen, Actividad, Aplicaciones, Recursos, USB, WiFi y Diagnóstico. Fecha UTC y sección en URL. Actividad conserva búsqueda y hora; software conserva vista, consulta y página; las tablas comparten búsqueda/paginación en URL. Métricas e inventario también conservan equipo seleccionado.
- Resumen: recursos actuales, última actividad, aplicaciones más usadas y contadores por sesión. Los contadores sin captura disponible no se convierten en cero. Los detalles técnicos y desglose de personas quedan desplegables.
- Aplicaciones distingue software confirmado por fuente, ventanas reportadas y detecciones heredadas. Las últimas no demuestran instalación actual.
- Tablas: 25 filas por página en colecciones consultadas, 50 en inventario paginado por servidor. USB/WiFi consultan hasta 250 eventos del día; actividad reciente hasta 100 registros. La búsqueda local no afirma cubrir datos fuera de esos límites.
- Diagnóstico: almacenamiento, red, recolectores, sesiones y tareas en desplegables con búsqueda/paginación. Historial diario también paginado; los gráficos horarios ofrecen una tabla de valores equivalente.
- Ayuda dentro de la aplicación (`/help`): incorporación, Mi equipo, conectividad, sesiones, captura, permisos, exportación y significado de los datos.

## Fallos y calidad de los datos

Las consultas del detalle y métricas se resuelven independientemente. Una caída de USB no bloquea recursos o actividad. Cada sección tiene reintento propio. Las respuestas tardías de consultas abandonadas se ignoran.

Al actualizar la misma consulta se conserva la última respuesta y se advierte si falla. Al cambiar equipo, fecha o consulta no se muestran filas de otro ámbito. Cargando, sin respuesta, consulta vacía y error son estados distintos. Los errores de exportación no vacían el detalle.

Las duraciones WiFi solo se calculan entre eventos consecutivos de una misma interfaz; sin un evento posterior se muestra «Sin cierre observado». No se extrapola una conexión antigua hasta el momento presente. No se denomina red actual al último evento de una fecha histórica.

Los días y horas de agregación son UTC. Las marcas de eventos se presentan en la zona del navegador y se identifican como tales. Tiempo por persona no equivale a encendido del equipo. Un reporte reciente no garantiza que la ventana siga abierta.

## Permisos y privacidad

El rol de consulta ve la exportación de actividad deshabilitada con explicación. Las acciones administrativas mantienen sus restricciones; un rechazo 403 produce un aviso comprensible sin sustituir el control del servidor. Cambiar la interfaz no concede permisos.

Un enlace conserva filtros, no autorización: requiere sesión y permisos. Las búsquedas pueden contener títulos/rutas sensibles; no compartir enlaces con esos términos ni capturas de credenciales. Las capturas de validación usan exclusivamente datos sintéticos.

## Accesibilidad y pantallas

Tema oscuro predeterminado, Claro y Sistema disponibles. Foco visible, enlace para saltar al contenido, foco en contenido al cambiar de página, menú móvil con cierre Escape/restauración de foco, regiones de tabla desplazables por teclado, encabezados de tabla, avisos de carga/error y alternativa textual a gráficos. Se respeta preferencia de movimiento reducido.

Comprobación focalizada en Chromium del build: 390, 768 y 1440 px, ambos temas, sin desbordamiento horizontal de página. Contraste mínimo 4,5:1 para los tokens principales de texto (`--ink`, `--muted`, `--accent`) frente a superficie. No equivale a auditoría completa WCAG de todos los estados/componentes.

## Evidencia reproducible

Desde dashboard ejecutar `npm run build`. Luego, desde la raíz: `python tests/dashboard_ux.py` (Python Playwright y Chromium instalados). El navegador intercepta assets compilados y respuestas API sintéticas; no usa el piloto ni certifica la integración real del backend. Capturas privadas en `.local-test/ux-review/`.

El escenario cubre fallos parciales, reintento local, retención de datos de la misma consulta, separación entre fechas, búsqueda/paginación, recarga/enlaces, retorno al listado, duración WiFi desconocida, rechazo de exportación, rol de consulta, diagnóstico, temas, tamaños y foco del menú. ESLint sobre archivos modificados y compilación TypeScript/Vite forman parte de la comprobación.

Pendiente de aceptación integrada: lector de pantalla real, recorridos con usuarios representativos, navegadores soportados y datos reales multiplataforma. Se registra en bloque 9; no quedan esos ensayos marcados como realizados por esta comprobación sintética.
