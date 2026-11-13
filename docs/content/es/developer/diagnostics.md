---
title: "Diagnósticos y evidencia segura"
docId: "developer.diagnostics"
translationKey: "developer.diagnostics"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "123301b5d60789ec9ef26ce98cb57270b2882860d3cdb13b52daa722d09c8794"
translationRevision: "123301b5d60789ec9ef26ce98cb57270b2882860d3cdb13b52daa722d09c8794"
order: 130
---

# Diagnósticos y evidencia segura

## Objetivo y requisitos

Recoge evidencia suficiente para localizar un fallo sin conservar secretos ni confundir diagnóstico con recuperación. Usa un host privado y conoce el [transporte](transports.md) seleccionado. La aplicación instala un único recolector por sesión; los componentes de estado llevan a él y no añaden entradas durante el renderizado.

## Inspeccionar un fallo

1. Abre la acción contextual del estado de la conexión, plugin, sincronización o actualizador afectado. El foco de teclado muestra la acción de registros igual que el cursor.
2. Filtra por origen, nivel y texto; inspecciona contexto y número de repeticiones.
3. Copia las entradas seleccionadas o filtradas que expliquen el problema. Revisa las exportaciones antes de compartirlas.
4. Compara la salud nativa con lo observado en el frontend:

```sh
target/debug/tiktools --json system info
target/debug/tiktools --json system health
target/debug/tiktools --json system doctor
```

La comprobación de LIVE desconectado puede omitirse. Una capacidad compilada no demuestra disponibilidad real. En escritorio el panel no es modal; las pantallas estrechas usan `/debug` con destino de regreso. La exportación nativa utiliza `system.saveFile`; el navegador descarga localmente.

## Entender la retención

`src/web/shared/diagnostics/log.ts` limita el historial a 2.000 entradas y 2 MiB de UTF-8 serializado. Eventos consecutivos iguales de un origen/entidad en cinco segundos se combinan con un contador. La ocultación ocurre antes de retener datos e incluye credenciales reconocibles y secretos de URL; no siempre detecta valores arbitrarios sin etiqueta de terceros. No añadas ajustes completos, cookies, parámetros RPC ni cuerpos HTTP al recolector.

## Resultado esperado y estado

El visor ofrece historial acotado de sesión y navegación contextual, no auditoría duradera. Recargar o desmontar libera la memoria. Borrar el historial no elimina el error de conexión ni repara su causa. Los identificadores de solicitud Cloud ayudan a correlacionar sobres seguros; la cobertura depende de los recolectores instalados y no garantiza capturar todo error nativo o de registro.

## Verificación y recuperación

Ejecuta `bun scripts/task.ts test web diagnostics` y consulta el [código del recolector](https://github.com/nglmercer/TikTools-app/tree/remake/src/web/shared/diagnostics). Reproduce ocultación, descarte y repeticiones con valores sintéticos. Ante conflictos, órdenes inciertas o credenciales vencidas, vuelve al flujo propietario de recuperación. Copiar un diagnóstico recoge evidencia; nunca repite acciones ni aprueba confianza nueva.
