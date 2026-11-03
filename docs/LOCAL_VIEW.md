# Mi equipo: vista local de solo lectura

El agente del sistema sirve una interfaz gráfica en http://127.0.0.1:9876/. Se abre con el navegador local, sin instalar Electron ni acceder al dashboard del servidor. El diseño usa los mismos colores y organización visual del dashboard de equipos de oficina.

Muestra únicamente este equipo: CPU, memoria, uptime, discos, tasas de red, cola pendiente y componentes supervisados. Las gráficas conservan hasta 60 muestras distintas observadas mientras la página está abierta; no almacenan históricos ni consultan otros equipos. Muestra fechas y avisa ante datos ausentes/antiguos o desconexión. Las muestras proceden del recolector existente: no duplica captura ni anula pausas administrativas.

Solo acepta GET. Se retiraron ayuda/desinstalación de esta interfaz. No expone credenciales, tokens, configuración administrativa, títulos de ventanas ni eventos de otros usuarios. La API local reducida no requiere el token administrativo: está limitada a loopback y verifica Host, Origin y contexto de origen del navegador. Sin CORS, sin scripts/fuentes externos, sin caché y sin permiso para incrustar la página en otro sitio. WEB_BIND debe ser una dirección IP loopback literal; se rechazan interfaces remotas.

Un usuario con acceso local al equipo puede consultar esta vista. No ofrece separación de hardware por usuario de Windows/Linux; CPU y memoria son del equipo. El endpoint administrativo anterior /api/status conserva Bearer obligatorio, sin token en URL. Los endpoints de mutación ya no están disponibles.

Acceso: enlace «Monitor — Mi equipo» previsto en DEB/MSI; tray de Windows del agente interactivo o de sesión. El servicio debe estar instalado y ejecutándose para obtener datos. La sesión no abre un segundo servidor HTTP ni accede a credenciales del servicio. El acceso directo usa el puerto predeterminado 9876; si se cambia WEB_BIND, abrir manualmente la dirección configurada.

Conectividad se presenta como disponibilidad del canal del publicador, no como confirmación de recepción de cada evento por el servidor. La supervisión de un componente tampoco equivale a éxito de cada captura.

Implementado en fuente. No se compiló, desplegó ni revisó visualmente en este bloque; esas comprobaciones permanecen en P4.
