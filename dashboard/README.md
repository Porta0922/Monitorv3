# ActivityMonitor Dashboard
*Actualizado: 28 de Abril, 2026*

Este es el frontend de la plataforma ActivityMonitor Enterprise v3, desarrollado en React con TypeScript y Vite.

## Características

- **Visualización en Tiempo Real**: Estado de conexión y actividad de los agentes.
- **Heatmaps de Actividad**: Intensidad de uso de dispositivos por hora y día.
- **Centro de Seguridad**: Monitoreo de alertas MITRE ATT&CK detectadas por osquery.
- **Gestión de Dispositivos**: Búsqueda avanzada y filtrado de endpoints.
- **Historial de Inventario**: Registro de software, redes WiFi y dispositivos USB.

## Tecnologías

- **Framework**: React 18
- **Lenguaje**: TypeScript
- **Estilos**: Tailwind CSS / Lucide React (Iconos)
- **Gráficos**: Recharts / Tremor
- **Herramienta de Build**: Vite

## Inicio Rápido

1. Instalar dependencias:
   ```bash
   npm install
   ```

2. Configurar variables de entorno:
   Crea un archivo `.env` en esta carpeta:
   ```env
   VITE_API_URL=http://localhost:3000
   ```

3. Ejecutar en desarrollo:
   ```bash
   npm run dev
   ```

## Build para Producción

Para generar los archivos estáticos optimizados:

```bash
npm run build
```

Los archivos se generarán en la carpeta `dist/`. Pueden ser servidos por Nginx o cualquier servidor de archivos estáticos.

## Estructura de Carpetas

- `src/components`: Componentes reutilizables de la UI.
- `src/pages`: Vistas principales (Dashboard, Dispositivos, Seguridad, etc.).
- `src/hooks`: Lógica de consumo de API y estados globales.
- `src/utils`: Funciones auxiliares y formateadores.
