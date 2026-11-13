---
title: "Runtimes de plugins y límites de confianza"
docId: "developer.plugins.runtimes"
translationKey: "developer.plugins.runtimes"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "8c9d406371879e3d0f5bf813d43af66dd12e24795f8dab281c395d42ba47939e"
translationRevision: "8c9d406371879e3d0f5bf813d43af66dd12e24795f8dab281c395d42ba47939e"
order: 260
---

# Runtimes de plugins y límites de confianza

## Objetivo y requisitos

Elige un formato ejecutable cuyas dependencias y confianza correspondan al host. Lee la [introducción](index.md), valida el manifiesto contra la compilación real e inspecciona restricciones de plataforma antes de cargar código. Compatibilidad de protocolo, ABI y permisos son comprobaciones independientes.

## Elegir un runtime implementado

| Runtime | Ejecución | Confianza y portabilidad |
| --- | --- | --- |
| `native` | ABI C mediante `libloading` | Código confiable dentro del proceso; específico del destino |
| `process` | Ejecutable independiente, stdin/stdout enmarcado | Aísla caídas; permisos normales del usuario |
| `napi-vm` | JavaScript anticipado sobre QuickJS con `rquickjs` | Efectos denegados por defecto; addons cambian la confianza |
| `declarative` | Descriptores de integración validados | Sin programa invitado; efectos por el host |
| `wasm` | Ranura opcional | Motor general no activado en compilaciones normales |

La ABI nativa v1 intercambia bytes serializados mediante una tabla C pequeña; no cruzan contenedores Rust, objetos de rasgo ni futuros. Las bibliotecas permanecen cargadas hasta cerrar. El proceso usa longitud u32 little-endian seguida de bytes JSON, no RPC por líneas. Evita escribir diagnósticos en stdout enmarcado.

## Entender la compatibilidad QuickJS

El nombre `napi-vm` permanece estable, pero las afirmaciones de un intérprete Rust separado son históricas. Los invitados reciben globales compatibles documentados, no Node completo. `napi_get_uv_event_loop` no está admitido. Los addons `.node` usan una ruta confiable explícita, permitida y verificada con SHA-256 dentro del proceso; el nombre del runtime JavaScript no los vuelve seguros.

```sh
bun scripts/task.ts rust test -p tiktools-plugin-loader --lib --locked
bun scripts/task.ts test bun packages
```

Usa pruebas dirigidas cuando la suite completa de addons requiera herramientas ausentes.

## Resultado esperado y estado

El analizador rechaza protocolo/ABI incompatibles antes de cargar. Android permite QuickJS portable y ejecutables de proceso Android/Bionic válidos, pero rechaza bibliotecas dinámicas y `.node`. Un artefacto Linux/glibc de escritorio no sirve como plugin de proceso Android.

## Recuperación

Reinicia un host privado tras reemplazar una biblioteca nativa. Si el inicio agota su plazo y queda incierto, el gestor conserva la identidad y bloquea sustituciones; no eludas ese estado declarando cierre correcto. Los buffers de eventos son acotados y pueden informar retraso; emitir durante inicio no constituye una cola duradera. La [implementación del cargador](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-plugin-loader/src) define motor y ciclo de vida actuales.
