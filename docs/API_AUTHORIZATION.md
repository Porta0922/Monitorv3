# Autorización HTTP

Las mismas reglas se aplican a las rutas antiguas y a su alias `/api`.

| Ruta | Credencial |
| --- | --- |
| `/health`, `/ready`, `/auth/login` | Públicas |
| `POST /agent/enroll` | Código de un solo uso válido para el UUID y nuevo token generado por el agente |
| Lectura de equipos, métricas, actividad e inventario | Sesión humana válida; las rutas administrativas mantienen restricciones adicionales |
| `/agent-credentials/*`, incluido enrolamiento/listado/revocación | Sesión de administrador |
| `POST /agent/commands` | Sesión de administrador |
| Políticas del agente, consulta de comandos, ACK y renovación | Credencial individual en `x-agent-token`; ámbito de UUID/propietario comprobado |
| `POST /devices/register`, `POST /logs/ingest` | Sesión de administrador o credencial individual del UUID solicitado |
| Resolver alertas e investigaciones | Sesión de operador/administrador |

El middleware rechaza accesos no autorizados antes de ejecutar la operación solicitada. Las credenciales individuales se consultan en DB para comprobar vencimiento/revocación; si falla esa consulta, no se concede acceso. En producción se prohíbe el token HTTP global. La compatibilidad con `AGENT_AUTH_TOKEN` compartido se limita al modo de desarrollo del piloto.

El despliegue candidato utiliza cuentas humanas administradas y secretos desde archivos; no admite ADMIN_PASSWORD de bootstrap. La identidad AMQP usa usuarios individuales y permisos por equipo, distintos de las cuentas del servidor y de administración del broker. Ver [identidad y recuperación](AGENT_IDENTITY.md) y [despliegue](PRODUCTION_DEPLOYMENT.md).

El dashboard utiliza cookie HttpOnly y CSRF en memoria, mediante `/api` del mismo origen. No guarda JWT en localStorage. En producción el Bearer humano se rechaza. Ver [cuentas y recuperación](ACCESS_CONTROL.md).

`/auth/session`, `/auth/sessions`, `/auth/logout` y `/auth/revoke-all` requieren sesión propia; `/auth/security` y `/auth/users/:username/sessions` son administrativos. `/export/csv` requiere operador o administrador; `/audit`, `/administration/*` y `/maintenance/*` requieren administrador. Los usuarios pueden revocar únicamente sus sesiones, salvo la ruta administrativa explícita.

`/health` indica que la API responde. `/ready` devuelve 503 hasta que la base
responda y se hayan establecido los consumidores RabbitMQ; Compose usa `/ready`.

P1: `/maintenance/status` y `/resources/:device_id/daily` requieren Sesión de administrador en ambos alias HTTP. Implementación pendiente de validación integrada.

`GET /devices/:device_id/sessions` ofrece contadores por sesión y totales del día UTC para lectores autenticados. `GET /devices/:device_id/sessions/export?day=YYYY-MM-DD` requiere operador/admin, registra auditoría y exporta el mismo agregado; máximo 10.000 sesiones, sin truncamiento silencioso. Mismos controles en `/api`. Equipo inexistente: 404; fecha/offset inválidos: 400; fallo de consulta: 500. CSV conserva vacíos para contadores sin muestras.
