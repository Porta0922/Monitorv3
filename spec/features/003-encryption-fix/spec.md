# 003 · Fix encoding XML tarea programada

**Estado:** implementado ✅

## Qué hace

Corrige el encoding de los archivos XML generados para `schtasks.exe` de UTF-16 y UTF-8 con BOM a ASCII (UTF-8 sin BOM), para que Windows los procese correctamente.

## Por qué

`schtasks.exe` rechaza XML con encoding UTF-16 (error de parsing) y UTF-8 con BOM. El estándar de Windows espera UTF-8 sin BOM o ANSI.

## Criterios de aceptación

- [x] `schtasks /create /XML ...` no falla con error de encoding.
- [x] El XML usa `-Encoding ASCII` en PowerShell.

## Fuera de alcance

- Cambiar el formato de los XML en sí.
