---
title: "Descargas y actualizaciones"
docId: "user.start.updates"
translationKey: "user.start.updates"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6b92351b3ee8eb1b0745db65bbddb3eab21f4ae0a9945795c94147c9ca347e73"
translationRevision: "98a7c2cfe052496880550d587bd75ee988e03d0ac01aa3ac71d3c769b7b0814e"
order: 40
---

# Descargas y actualizaciones {#downloads-and-updates}

## Objetivo y requisitos {#goal-and-prerequisites}

Elegir el canal correcto y actualizar sin confundir la búsqueda de versiones con una instalación automática. Cierra las sesiones activas y respalda los datos locales antes de cambiar de versión, especialmente si sus notas documentan una migración de base de datos.

## Elige un canal {#select-a-channel}

La [lista de versiones](https://github.com/nglmercer/TikTools-app/releases) distingue versiones estables y preliminares. Un candidato como `-rc` sirve para verificación. El actualizador distingue los feeds estable y beta y autentica sus metadatos firmados antes de ofrecer una versión. Un JSON sin firma o un instalador modificado no constituye una actualización válida.

## Buscar e instalar {#check-and-install}

1. Abre **Settings → General** y comprueba la versión instalada y el estado del actualizador.
2. Ejecuta la comprobación disponible. Si la compilación no configura la clave pública o el feed, mostrará que esa capacidad no está disponible.
3. Lee las notas y elige el paquete de tu plataforma.
4. Sigue la descarga o instalación manual que ofrezca tu compilación. El workflow habilita comprobaciones autenticadas solo si está configurada la variable de clave pública; la instalación automática tiene puertas adicionales.
5. Para actualizar manualmente, cierra TikTools, instala o sustituye el paquete completo verificado y vuelve a abrirlo. Conserva los datos fuera de los archivos de la aplicación.

## Resultado esperado {#expected-result}

La aplicación reiniciada informa de la versión elegida y abre el perfil local esperado. Los ZIP/TAR portátiles requieren sustitución manual. Un ejecutable extraído o una instalación no reconocida también puede requerir ese procedimiento aunque exista un instalador AppImage.

## Recuperación y límites {#recovery-and-limitations}

Una firma inválida significa que la actualización falló: descarga una copia oficial nueva y verifica sus checksums. No omitas la verificación. El reemplazo automático de AppImage y la entrega a NSIS tienen gates de compilación separados y son experimentales. No existe un rollback automático general. Volver a un binario anterior no deshace una migración SQLite.

Reporta la versión, el formato del paquete y el error del actualizador desde Diagnostics. No adjuntes claves de firma, tokens de invitación ni datos de cuenta. La [guía de instalación](installation.md) describe los formatos compatibles.

## Detalles de operación {#operational-details}

La capacidad del updater se fija al compilar. Comprobar requiere feed y clave pública. AppImage automático exige además puerta común de instalación y origen compatible detectado; NSIS exige esa misma puerta Y la específica NSIS. Variables de entorno después de instalar no habilitan puertas compiladas. Portable/extraído/desconocido usa reemplazo manual. Verifica versión, reinicio y perfil tras instalar; descarga, checksum o lanzar instalador no prueban finalización.
