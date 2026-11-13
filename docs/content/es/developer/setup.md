---
title: "Preparación y ciclos de desarrollo"
docId: "developer.setup"
translationKey: "developer.setup"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d907aca4a6ab33f86bdd51498fd7607322ebbff9daefb3338bd0c004c06dbfb1"
translationRevision: "d907aca4a6ab33f86bdd51498fd7607322ebbff9daefb3338bd0c004c06dbfb1"
order: 10
---

# Preparación y ciclos de desarrollo

## Objetivo y requisitos

Compilar el código actual de `remake` sin modificar el perfil instalado del usuario. Utiliza Bun 1.4.4, indicado en `package.json`, Rust 1.95 o posterior y un Node compatible con el Vite del lockfile: `^20.19.0 || >=22.12.0`. CI usa actualmente Rust 1.99.1. Compilar es distinto de [instalar un binario](../user/installation.md).

## Preparación por plataforma

En Windows instala MSVC Build Tools, WebView2 y Strawberry Perl nativo para OpenSSL vendorizado. El wrapper de Cargo detecta MSVC y Perl nativo, respetando los overrides explícitos `OPENSSL_SRC_PERL`/`PERL`. El Perl de MSYS/Cygwin no sirve para esta ruta MSVC. LLVM `lld-link` es opcional.

Para desarrollar el escritorio en Ubuntu 24.04, CI instala `libasound2-dev`, `libappindicator3-dev`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libxdo-dev` y `pkg-config`. Empaquetar AppImage también requiere los metadatos de desarrollo de GStreamer, sus plugins base/good/libav y el scanner. `mold` acelera opcionalmente el enlazado Linux. Android/Termux utiliza un toolchain Android nativo, no esas bibliotecas de escritorio.

## Instalar dependencias

```sh
git switch remake
bun ci
```

Conserva los lockfiles de Bun y Cargo. Utiliza `bun install` sólo al cambiar dependencias intencionalmente. La documentación es un workspace explícito: VitePress estable tiene su dependencia Vite aislada y la aplicación conserva Vite 8. No fuerces ambos a una sola versión mayor.

## Elige un ciclo

```sh
bun run start                 # Vite y host de escritorio Wry de desarrollo
bun run dev                   # desarrollo en navegador local, sin preparar plugins de ejemplo
bun scripts/task.ts serve     # sólo Vite; las operaciones nativas necesitan host/adaptador real
bun scripts/task.ts packaged  # frontend empaquetado, sin URL de desarrollo
```

`start` inicia Vite, espera su URL asignada y ejecuta el escritorio con `TIKTOOLS_DEV_URL`. Puede preparar plugins desde el checkout externo `plugins-example-tiktools`, encontrado mediante `TIKTOOLS_PLUGINS_REPO` o un directorio hermano. No compila implementaciones de plugins incluidas en este repositorio. Usa `TIKTOOLS_SKIP_DEV_PLUGINS=1` para omitir esa preparación. Un host existente no se sustituye: el arranque informa de un propietario abierto.

Para pruebas nativas aisladas, asigna a `TIKTOOLS_HOME` un directorio descartable antes de iniciar el host. Utiliza el mismo directorio desde las terminales CLI que lo controlan. Nunca elimines un bloqueo de propietario activo.

## Validar y recuperar

```sh
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts rust fmt --all -- --check
bun scripts/task.ts rust-lint
bun scripts/task.ts rust test --workspace --locked -- --test-threads=1
```

Ejecuta pruebas enfocadas al iterar y conserva todos los gates obligatorios de CI antes de integrar. Evita varias compilaciones Cargo sobre el mismo target mientras se cronometran tests de firma que invocan Cargo. Si el arranque detecta un host anterior, detén su proceso limpiamente; si falta un enlazador o una biblioteca WebView, corrige el entorno antes de cambiar la lógica de la aplicación.
