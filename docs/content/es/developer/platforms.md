---
title: "Plataformas y compilación nativa en Termux"
docId: "developer.platforms"
translationKey: "developer.platforms"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "c7befe2be6f91bcf50ce4572aaa95d02723a315ba11ed1d94e6d499e802c2473"
translationRevision: "f162187f6990abeef33432d132499e4c4d7c2892298e85e67c855c89f33fa72a"
order: 20
---

# Plataformas y compilación nativa en Termux {#platforms-and-native-termux-builds}

## Objetivo y requisitos {#goal-and-prerequisites}

Elige una compilación que corresponda al sistema de destino. El desarrollo de escritorio requiere Bun 1.4.4, Rust 1.95 o posterior y las dependencias WebView indicadas en [preparación](setup.md). Termux compila la CLI de Rust directamente en el directorio privado de Android; el almacenamiento compartido no sirve para ejecutables ni sockets Unix.

## Compilar el host adecuado {#build-the-appropriate-host}

Windows utiliza MSVC Build Tools y WebView2. Strawberry Perl nativo permite compilar OpenSSL incluido; no lo sustituyas por Perl de MSYS. El escritorio Linux necesita GTK3, WebKitGTK 4.1, bibliotecas de desarrollo de audio y `pkg-config`. La integración de Winit/Wry pertenece a `crates/tiktools-desktop/src/platform.rs`; los servicios del núcleo permanecen independientes de la ventana.

En un teléfono ARM64, comprueba primero que `rustc -vV` indique `aarch64-linux-android`. En Termux:

```sh
pkg install git rust clang make cmake pkg-config python protobuf ca-certificates
export CARGO_BUILD_JOBS=2
bash scripts/termux/check.sh
cargo build -p tiktools-cli --locked
./target/debug/tiktools --standalone system info
```

Aquí Cargo es deliberado: el flujo nativo del teléfono no requiere Bun ni Node. En un escritorio con Bun, ejecuta Cargo mediante `bun scripts/task.ts rust ...` para conservar la configuración del enlazador y la caché. Para usar dos terminales del teléfono, inicia `./target/debug/tiktools host --ipc` y conserva el mismo `TIKTOOLS_HOME` en las llamadas siguientes.

## Resultado esperado y estado {#expected-result-and-status}

IPC local, SQLite y los plugins portables de proceso/QuickJS tienen rutas Android implementadas. Los ejecutables de proceso deben dirigirse a Android/Bionic. Android no admite bibliotecas nativas ni complementos `.node`. Audio, atajos, notificaciones, selectores de archivos y WebView de escritorio carecen de backend telefónico. Los perfiles remotos persistidos fallan explícitamente porque no hay almacén de credenciales seleccionado; el SDK puede conservarlas en memoria.

## Verificación y recuperación {#verify-and-recover}

Ejecuta `bash scripts/termux/doctor.sh`, `bash scripts/termux/test.sh` y `bash scripts/termux/smoke.sh` desde la raíz del checkout. Indica las comprobaciones de frontend omitidas. Ante símbolos C++ ausentes en `protoc`, compara `env -u LD_LIBRARY_PATH protoc --version` antes de cambiar bibliotecas. Repara un almacén CA ausente sin desactivar su validación. La compilación cruzada no demuestra aceptación en el teléfono. La [script de validación de Termux](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/termux/smoke.sh) detalla las pruebas y restricciones.

## Detalles de operación {#operational-details}

Para navegador local, construye assets con index.html y CLI con `local-web`. Retira overrides heredados y usa `TIKTOOLS_HOME` descartable; inicia primero host IPC. En otra terminal con la misma raíz ejecuta el adaptador, abre enlace loopback o introduce código de seis dígitos de un uso (cinco minutos). Detén adaptador y host por separado. Loopback/pairing no ofrecen acceso remoto/LAN. Termux necesita toolchain Android nativo y assets web; binario Linux desktop no es Android.

```sh
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-cli --features local-web --locked
export TIKTOOLS_HOME=/absolute/path/disposable-home
target/debug/tiktools host --ipc
```

```sh
export TIKTOOLS_HOME=/absolute/path/disposable-home
target/debug/tiktools web --assets /absolute/path/to/TikTools/dist/web --port 4174
```
