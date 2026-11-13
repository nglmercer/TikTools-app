---
title: "Alcance de Termux y del modo sin interfaz"
docId: "user.start.termux"
translationKey: "user.start.termux"
locale: "es"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a823f6d8f669895af6858723159b7eb4fea1fbaf165102b61396b4d2e90efa19"
translationRevision: "a823f6d8f669895af6858723159b7eb4fea1fbaf165102b61396b4d2e90efa19"
order: 50
---

# Alcance de Termux y del modo sin interfaz

## Objetivo y requisitos

Entender qué partes de TikTools funcionan sin una WebView de escritorio. Es un flujo avanzado, no una versión Android del instalador Windows/Linux. El proyecto dispone de CLI nativa Rust para Termux y host sin interfaz; obtener un binario adecuado o compilarlo corresponde a la preparación de desarrollo.

## Utiliza un solo host

1. Inicia un host sin interfaz con `tiktools host --ipc` en el entorno donde instalaste la CLI.
2. Mantén el mismo directorio de aplicación entre terminales.
3. Desde otra terminal, usa `tiktools system info`, `tiktools live status` o `tiktools api discover` para inspeccionar ese host.
4. Detén el proceso limpiamente con el apagado documentado o Ctrl-C. No elimines bloqueos de propietario mientras esté ejecutándose.

Los comandos normales se conectan al host existente. `--standalone` crea explícitamente otro núcleo para una invocación; no es el modo predeterminado de controlar un host abierto.

## Vista previa en navegador local

El adaptador opcional `local-web` permite servir la interfaz Vue a un navegador después del emparejamiento local. Es una vista previa de desarrollo con requisitos de compilación y ciclo de vida propios. No convierte Android en un destino de WebView de escritorio compatible ni concede acceso remoto a otra persona.

## Resultado esperado

CLI y escritorio comparten conceptos de control, persistencia SQLite y procesamiento nativo de eventos. Los plugins dependientes de plataforma, audio, atajos, ventanas nativas y almacenes de credenciales pueden no estar disponibles. La persistencia de perfiles remotos tiene restricciones explícitas en Android.

## Recuperación

Utiliza el directorio privado de Termux en lugar del almacenamiento compartido de Android, que puede impedir la ejecución y los sockets Unix. Conserva el mismo directorio de aplicación y comprueba que el host está activo antes de investigar un fallo IPC. Si Android detiene el proceso, reinícialo con ese perfil y revisa los diagnósticos de recuperación. La demo documental no reproduce el ciclo de vida de Android.

La [guía de desarrollo](../developer/index.md) contiene el toolchain, los feature flags y la verificación en el dispositivo. La instalación de escritorio sigue en la [guía Windows/Linux](installation.md).
