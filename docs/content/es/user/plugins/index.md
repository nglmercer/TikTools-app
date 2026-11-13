---
title: "Instalar y administrar plugins"
docId: "user.plugins.index"
translationKey: "user.plugins.index"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "93c8811682d0baa5bd18b571503fce483cf1f464e94b94bc933241bf277d23d4"
translationRevision: "93c8811682d0baa5bd18b571503fce483cf1f464e94b94bc933241bf277d23d4"
order: 10
---

# Instalar y administrar plugins {#install-and-manage-plugins}

## Objetivo y requisitos {#goal-and-prerequisites}

Añade una integración, revisa el acceso solicitado y confirma que sus acciones funcionan. Necesitas un paquete de escritorio compatible con el sistema operativo, arquitectura y versión del host exigidos por el plugin. El catálogo remoto y las descargas requieren Internet; algunos servicios también necesitan [sesión de Cloud](../cloud/account.md).

## Revisar antes de activar {#review-before-enabling}

1. Abre **Plugins** y consulta Installed, Explore o Updates. Explore solicita únicamente plugins. Las plantillas y perfiles se encuentran en [Automations](../automations/templates.md).
2. Selecciona un plugin para revisar descripción, versión, compatibilidad y entorno de ejecución. Comprueba editor, capacidades y permisos; el nombre visible por sí solo no demuestra confianza.
3. Usa el flujo de instalación para inspeccionar la vista previa del paquete. Revisa versión exacta, información de firma, acceso solicitado y diferencias respecto a la versión instalada antes de confirmar.
4. Espera el resultado del trabajo de instalación. Descargar, instalar y activar son etapas distintas; una tarjeta del catálogo no significa que el plugin esté preparado.
5. Abre sus paneles Settings o Connection. Introduce dirección y credenciales únicamente en los campos previstos. Usa la comprobación de conexión declarada si existe.
6. Activa el plugin y comprueba si aparecen sus acciones o páginas. Crea una [regla de registro](../automations/first-rule.md) limitada antes de vincular una acción externa potente.

Los plugins pueden ejecutar código nativo o procesos separados. Un proceso puede aislar un fallo, pero no constituye una zona aislada del sistema operativo. Instala software de editores confiables. Una firma válida identifica la procedencia del paquete revisado; no garantiza que todos sus efectos sean apropiados para tu flujo.

## Resultado esperado y estado {#expected-result-and-status}

Están disponibles la revisión de paquetes, los trabajos de instalación, las preferencias y la gestión de versiones. Las acciones concretas dependen del manifiesto instalado, las capacidades del host y el servicio externo. Los procesadores de eventos tienen un panel propio y pueden añadir campos de análisis opcionales; los campos originales siguen siendo distintos.

## Recuperación {#recovery}

Ante un fallo, lee el error del trabajo para identificar incompatibilidad, firma, entorno o descarga. Evita debilitar la verificación para eliminar un aviso. Si una actualización falla, revisa las versiones conservadas y usa la reversión cuando esté disponible. Comprueba automatizaciones dependientes antes de desactivar o desinstalar. Conserva [copias](../preferences/backups.md) de datos y configuración antes de eliminarlos.

<DemoFrame scenario="plugins" locale="es" tour="plugin" title="Demo de plugins" />
