---
title: "Empaquetado, firmas y registros de plugins"
docId: "developer.plugins.packaging-registry"
translationKey: "developer.plugins.packaging-registry"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3d15fcb4c2e2528f60cab585a24e6d7172a4a4ca53905af1bc44cab5284edbcc"
translationRevision: "3d15fcb4c2e2528f60cab585a24e6d7172a4a4ca53905af1bc44cab5284edbcc"
order: 280
---

# Empaquetado, firmas y registros de plugins {#plugin-packaging-signing-and-registries}

## Objetivo y requisitos {#goal-and-prerequisites}

Distribuye un paquete cuyos bytes, autoridad del editor y compatibilidad se comprueben por separado. Parte de un plugin validado y un destino admitido. Firmar requiere claves privadas fuera del repositorio; delegar desde la raíz del registro es responsabilidad distinta del operador.

## Preparar un archivo revisable {#prepare-a-reviewable-archive}

```sh
tiktools-plugin validate --publish
tiktools-plugin build --release
tiktools-plugin keygen --id acme-1 --publisher acme --out /private/path/keys
```

`build` compila el paquete y crea archivos `.plugin`. Incluye entrada, recursos, idiomas, interfaz y dependencias pertinentes junto a `checksums.json`. Nativo/proceso necesitan artefactos compatibles; JavaScript permanece portable hasta que addons o funciones de plataforma lo restrinjan.

Para un archivo ya construido, usa rutas exactas en lugar de comodines sin revisar:

```sh
tiktools-plugin sign dist-plugin/acme.hello-0.1.0.plugin --key /private/path/keys/acme-1.key.json
tiktools-plugin verify dist-plugin/acme.hello-0.1.0.plugin --trust /private/path/keys/acme-1.pub.json
```

Sustituye el nombre por la salida real, incluida su plataforma si corresponde. La firma cubre las sumas, que cubren los archivos. Las claves públicas pueden compartirse; las privadas no deben llegar al código ni a registros.

## Confianza y política de actualización {#trust-and-update-policy}

El protocolo estático utiliza autoridad firmada por raíz y claves delegadas de índice/editor. Aprobar un espacio de nombres no equivale a verificar integridad; un resumen del catálogo Cloud no reemplaza ninguna cadena. El resolvedor comprueba canal, motor, esquema/protocolo/ABI, runtime y destino. Versiones retiradas, revocadas o incompatibles siguen excluidas aunque coincida su hash.

La instalación muestra primero una revisión. Actualizar automáticamente exige política permitida, paquete sin fijar, editor firmado compatible y ausencia de autoridad nueva; ampliar permisos requiere revisión. Instalar desde catálogo Cloud es distinto de operar registros estáticos y el producto rechaza URLs remotas arbitrarias.

## Resultado esperado, recuperación y verificación {#expected-result-recovery-and-verification}

El instalador verifica bytes antes de aplicar el paquete revisado y conserva estado de reversión/recuperación. La preparación de desarrollo queda fuera de su gestión. Ante fallos de confianza, corrige delegación o metadatos sin adoptar anulaciones de firma como política habitual. Si el inicio queda incierto, detén sustituciones y reconcilia el resultado.

Usa `bun scripts/task.ts rust test -p tiktools-plugin-package --locked` y pruebas de registro con muestras locales. El [verificador](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-plugin-package/src) y [modelo de registro](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-plugin-registry/src/model.rs) poseen los formatos; la infraestructura de publicación necesita aceptación separada.
