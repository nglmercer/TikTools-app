---
title: "Descargas y actualizaciones"
docId: "user.start.updates"
translationKey: "user.start.updates"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "53a9e9209749f0d6ea58737a3a2eadf871ad344e41fa17cd0e363cf43e181b3f"
translationRevision: "53a9e9209749f0d6ea58737a3a2eadf871ad344e41fa17cd0e363cf43e181b3f"
order: 40
---

# Descargas y actualizaciones

## Objetivo y requisitos

Elegir el canal correcto y actualizar sin confundir la búsqueda de versiones con una instalación automática. Cierra las sesiones activas y respalda los datos locales antes de cambiar de versión, especialmente si sus notas documentan una migración de base de datos.

## Elige un canal

La [lista de versiones](https://github.com/nglmercer/TikTools-app/releases) distingue versiones estables y preliminares. Un candidato como `-rc` sirve para verificación. El actualizador distingue los feeds estable y beta y autentica sus metadatos firmados antes de ofrecer una versión. Un JSON sin firma o un instalador modificado no constituye una actualización válida.

## Buscar e instalar

1. Abre **Settings → General** y comprueba la versión instalada y el estado del actualizador.
2. Ejecuta la comprobación disponible. Si la compilación no configura la clave pública o el feed, mostrará que esa capacidad no está disponible.
3. Lee las notas y elige el paquete de tu plataforma.
4. Sigue la descarga o instalación manual que ofrezca tu compilación. El workflow de producción habilita comprobaciones autenticadas; no activa todas las funciones experimentales de instalación automática.
5. Para actualizar manualmente, cierra TikTools, instala o sustituye el paquete completo verificado y vuelve a abrirlo. Conserva los datos fuera de los archivos de la aplicación.

## Resultado esperado

La aplicación reiniciada informa de la versión elegida y abre el perfil local esperado. Los ZIP/TAR portátiles requieren sustitución manual. Un ejecutable extraído o una instalación no reconocida también puede requerir ese procedimiento aunque exista un instalador AppImage.

## Recuperación y límites

Una firma inválida significa que la actualización falló: descarga una copia oficial nueva y verifica sus checksums. No omitas la verificación. El reemplazo automático de AppImage y la entrega a NSIS tienen gates de compilación separados y son experimentales. No existe un rollback automático general. Volver a un binario anterior no deshace una migración SQLite.

Reporta la versión, el formato del paquete y el error del actualizador desde Diagnostics. No adjuntes claves de firma, tokens de invitación ni datos de cuenta. La [guía de instalación](installation.md) describe los formatos compatibles.
