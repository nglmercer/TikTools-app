# Diagnóstico — capturas actuales

Código capturado: **cff05952bc9a7f8560ebff29c92f9d4c6be307c8**. Fecha: 2026-11-09. El commit posterior de esta galería solo añade imágenes y documentación. Frontend de producción y Chromium real contra un host temporal nativo sin cambios de backend. No hay mocks ni registros inyectados.

| Estado | Escritorio 1440 × 1000 | Móvil 390 × 844 |
| --- | --- | --- |
| Vista compacta | [Captura](overview-desktop-es.png) | [Captura](overview-mobile-es.png) |
| Filtros desplegados | [Captura](filters-desktop-es.png) | [Captura](filters-mobile-es.png) |
| Detalle de una entrada | [Captura](entry-desktop-es.png) | [Captura](entry-mobile-es.png) |
| Búsqueda sin resultados | [Captura](empty-filter-desktop-es.png) | [Captura](empty-filter-mobile-es.png) |

Se capturaron los registros reales de estado inicial desconectado y actualizador no disponible del host. El mensaje técnico se conserva en su idioma original; las etiquetas y acciones están traducidas al español. Las capturas son del viewport completo, sin montaje ni alteración.

Comprobado con Playwright: filtros y entradas por teclado, Escape/foco, conservación de búsqueda al minimizar/restaurar, copia real al portapapeles, descarga TXT, búsqueda sin resultados y restablecimiento, retorno móvil, cero errores de página y desbordamiento horizontal del documento. [Hashes de imágenes](manifest.json).

No se simuló un error de un servicio real, ni se insertaron mensajes sintéticos para rellenar el historial. Redacción de credenciales, fallback de copia, selección antigua y otros estados se prueban con tests separados. No se probó el diálogo nativo de guardar ni un lector de pantalla de sistema operativo. Automatizaciones no se modificó.
