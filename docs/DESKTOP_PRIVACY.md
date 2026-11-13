# Privacidad del escritorio

Implementación del candidato, no instalada en el piloto. Administración permite
configurar por política global o por equipo:

- `privacy_hide_titles`: reemplazar todos los títulos por `[Título protegido]`.
- `privacy_hide_paths`: omitir `exe_path` de las ventanas abiertas.
- `privacy_excluded_apps`: nombres exactos de aplicaciones, sin distinguir
  mayúsculas; desaparecen de las instantáneas de ventanas abiertas. La actividad
  conserva su duración bajo `[Aplicación excluida]`, sin título identificable.
- `privacy_title_filters`: frases literales contenidas en títulos, sin distinguir
  mayúsculas. No son expresiones regulares; hasta 100 reglas de 256 bytes por lista.

Los valores predeterminados mantienen el comportamiento anterior para no cambiar
silenciosamente el alcance acordado. Un administrador debe elegir los mínimos
necesarios y registrar el motivo del cambio. Las políticas antiguas se completan
con los valores predeterminados; una política parcial o inválida se rechaza.

El agente filtra antes de guardar eventos nuevos en su outbox, al recibirlos por
IPC en el servicio y antes de reenviar pendientes por IPC/RabbitMQ. Así, un corte
de red no omite las reglas vigentes al reanudar el transporte. IDs, timestamps y
duraciones permanecen estables. Los logs de intervalos de actividad ya no incluyen
nombres de aplicaciones y un nombre desconocido no se sustituye por el título.

No se modifican los registros históricos del servidor ni se purga físicamente la
caché cifrada anterior. Las filas confirmadas se limpian conforme al mantenimiento
existente. No se garantiza recuperar la identidad real de nombres ambiguos enviados
por versiones antiguas. Los filtros no cubren inventario, USB, WiFi o resultados de
osquery: desactivar sus recolectores cuando no estén autorizados. Ocultar rutas no
oculta hashes de ejecutables. No es un detector universal de secretos o datos personales.

Desplegar primero servidor/dashboard y después el agente compatible. La pantalla
indica si el agente ha confirmado la misma política; versiones anteriores ignoran
campos desconocidos, por lo que guardar la política no demuestra su aplicación.
El nuevo instalador debe tener una versión distinta de la 0.1.0 instalada, conforme
a VERSIONING.md. No se ha generado ni distribuido un nuevo paquete en este bloque.

Antes del release: aprobar finalidad, aviso a usuarios, roles de consulta, reglas
y retención; validar la política sobre equipos reales. Las pruebas focalizadas
comprueban redacción, exclusión, conservación de duración/ID, idempotencia y límites.
