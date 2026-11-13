---
title: "Configurar sonidos y voz"
docId: "user.plugins.audio"
translationKey: "user.plugins.audio"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "98dd80d7cc8f871e7639d38108e84ed0d15315113ead368f07a24c57e3c10ee8"
translationRevision: "98dd80d7cc8f871e7639d38108e84ed0d15315113ead368f07a24c57e3c10ee8"
order: 20
---

# Configurar sonidos y voz

## Objetivo y requisitos

Reproduce un sonido previsible antes de conectarlo a un LIVE concurrido. Necesitas un dispositivo de salida funcional y un archivo de audio compatible. **Play a sound** (`audio.play`) es una acción integrada; los proveedores de voz, como una integración SonicBoom instalada, requieren configurar su propio plugin y servicio.

## Probar un sonido local

1. En **Automations → Actions**, crea **Play a sound** (`audio.play`). Selecciona un archivo existente mediante el selector de medios.
2. Establece el volumen en `0.2` para la primera comprobación. El intervalo admitido va de `0` a `1`; cero significa silencio.
3. Elige la política de solapamiento: `allow` permite reproducción simultánea, `restart` reinicia el sonido y `drop` ignora una nueva instancia mientras ya se reproduce.
4. Guarda y lee la configuración antes de pulsar **Test**. La prueba puede producir sonido audible.
5. Escucha en el dispositivo previsto. Tras comprobar la reproducción local, vincula la acción a un evento con [filtro y espera](../automations/recipes.md) conservadores.

La acción conserva una referencia a la ruta del archivo. TikTools no copia el audio a su base de datos. Mantén el archivo en esa ubicación e inclúyelo por separado cuando traslades o respaldes la configuración.

## Configurar voz opcional

Instala el plugin de voz confiable desde [Plugins](index.md). En sus controles de conexión y ajustes, verifica dirección del servicio, autenticación, proveedor, idioma y voz o perfil. Carga el catálogo antes de probar una frase breve sin datos sensibles. Investiga una voz ausente como posible problema de catálogo, propiedad o proveedor; no inventes su identificador.

En SonicBoom, la síntesis, la admisión a reproducción y la finalización audible son resultados distintos. Una respuesta en cola o iniciada no demuestra que alguien escuchara la frase. La voz clonada puede tardar más que la llegada de mensajes: revisa diagnósticos de cola y saturación y limita los eventos admitidos.

## Resultado esperado, estado y recuperación

El audio local está disponible cuando el host anuncia su capacidad. La voz depende del plugin instalado y del servicio accesible; esta guía no garantiza aceptación de cada proveedor, dispositivo o plataforma. Si no suena, comprueba archivo, volumen, salida, pausa y error de ejecución. Para voz, verifica servicio y catálogo antes de repetir. Evita pruebas repetidas durante una emisión y consulta [diagnósticos](../preferences/troubleshooting.md) sin compartir tokens ni texto privado hablado.
