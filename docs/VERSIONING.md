# Versionado y mantenimiento de documentación

Actualizado: 2026-10-28.

## Serie del agente

La nueva serie empieza en **0.1.0** (la versión 0.1 solicitada, expresada con tres números para Cargo y MSI). Es una versión de desarrollo, no una declaración de estabilidad. La fuente de verdad es `agent/Cargo.toml`; el registro del paquete en `Cargo.lock` debe coincidir. El agente informa esa versión mediante `CARGO_PKG_VERSION` en arranque, salud y eventos.

- `0.1.1`, `0.1.2`: correcciones compatibles de una versión entregada.
- `0.2.0`, `0.3.0`: nuevos incrementos funcionales o cambios incompatibles durante el desarrollo. Documentar migraciones aunque el número mayor siga en cero.
- `1.0.0`: primera versión estable únicamente cuando se cumplan los criterios de salida del TODO. No hay una cantidad obligatoria de versiones intermedias ni una fecha de estabilidad implícita.

No incrementar por cada edición de un archivo. Agrupar cambios no publicados en la versión en desarrollo; incrementar antes de entregar un nuevo binario/paquete que modifique al agente o su instalación. Nunca reemplazar un artefacto entregado por otro distinto con el mismo identificador de release. Registrar commit, destino, versión, hashes y entorno de construcción en el manifiesto.

El servidor, dashboard y otros crates mantienen sus versiones independientes. No cambiar la versión del agente por una modificación exclusivamente visual del dashboard. La versión del protocolo/esquema de eventos tampoco es la versión del agente: evolucionarla explícitamente cuando cambie el contrato.

## Transición del piloto anterior

El agente instalado y el ZIP privado del piloto pueden seguir informando `3.4.0`. No se renombraron ni recompilaron esos artefactos al iniciar la serie 0.1.0. Los datos históricos conservan su versión original.

MSI interpreta 0.1.0 como inferior a 3.4.0. No desactivar a ciegas la protección de downgrade ni borrar identidad, configuración, claves o cola para forzar la instalación. Preparar y ensayar una migración coordinada del piloto a la nueva serie, con backup y recuperación. La estrategia para identidad de producto MSI debe definirse y comprobarse antes de distribuir 0.1.0 a equipos con 3.x. También comprobar comparación de versiones en políticas y scripts Linux.

El instalador local acepta nombres numéricos versionados y sigue comprobando el SHA256 del manifiesto. Eso no equivale a firma de producción ni resuelve por sí solo la transición MSI.

## Checklist por cambio

1. Actualizar README cuando cambien arquitectura, requisitos, comandos, configuración, capacidades, límites o estado de entrega.
2. Actualizar el documento específico afectado y el TODO vigente; conservar la evidencia histórica sin reescribirla como si correspondiera al nuevo código.
3. Registrar el cambio del agente/paquete en CHANGELOG; decidir si sigue en la versión no publicada o requiere la siguiente.
4. Antes de construir una entrega, verificar coincidencia entre Cargo.toml, Cargo.lock, versión del paquete, manifiesto y versión que informa el binario. Nunca emparejar un binario antiguo con un nombre nuevo.
5. Anotar validaciones realmente ejecutadas, fallos conocidos, compatibilidad y rollback. Actualizar README/CHANGELOG en el mismo conjunto de cambios.
6. Publicar tags de agente con formato `agent-v0.1.0` únicamente al autorizar una release. Documentar por separado el commit del servidor/dashboard compatible.

El README es la entrada vigente, no un historial ni una promesa de funciones aún pendientes. La sección de estado debe distinguir código preparado, paquete construido, instalación probada y producción aprobada.

## Piloto limpio instalado — 2026-11-03

El usuario eligió descartar el piloto 3.4.0 y sus datos. Se desinstaló el producto anterior y se instaló un MSI 0.1.0 nuevo, sin desactivar la protección de downgrade. Versión reportada y hashes comprobados. Esto resuelve esta instalación limpia; la migración que conserva datos sigue sin certificarse. Paquete privado en `.local-test/candidate-0.1.0/`, no redistribuible.
