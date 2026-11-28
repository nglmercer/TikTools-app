---
title: "Configurar sonidos y voz"
docId: "user.plugins.audio"
translationKey: "user.plugins.audio"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "116cd7a6c7a48b62cb5ec92b41e8bc3869652ef552f920a92b97722c68779b9d"
translationRevision: "18d5a891f129cb5a242500ff8906f2e4da349ba268f0143e54bfd3ae0c0227f6"
order: 20
---

# Configurar sonidos y voz {#set-up-sounds-and-speech}

## Objetivo y requisitos {#goal-and-prerequisites}

Reproduce un sonido previsible antes de conectarlo a un LIVE concurrido. Necesitas un dispositivo de salida funcional y un archivo de audio compatible. **Play a sound** (`audio.play`) es una acción integrada; los proveedores de voz, como una integración SonicBoom instalada, requieren configurar su propio plugin y servicio.

## Probar un sonido local {#test-a-local-sound}

1. En **Automations → Actions**, crea **Play a sound** (`audio.play`). Selecciona un archivo existente mediante el selector de medios.
2. Establece el volumen en `0.2` para la primera comprobación. El intervalo admitido va de `0` a `1`; cero significa silencio.
3. Elige la política de solapamiento: `allow` permite reproducción simultánea, `restart` reinicia el sonido y `drop` ignora una nueva instancia mientras ya se reproduce.
4. Guarda y lee la configuración antes de pulsar **Test**. La prueba puede producir sonido audible.
5. Escucha en el dispositivo previsto. Tras comprobar la reproducción local, vincula la acción a un evento con [filtro y espera](../automations/recipes.md) conservadores.

La acción conserva una referencia a la ruta del archivo. TikTools no copia el audio a su base de datos. Mantén el archivo en esa ubicación e inclúyelo por separado cuando traslades o respaldes la configuración.

## Configurar voz opcional {#configure-optional-speech}

Instala el plugin de voz confiable desde [Plugins](index.md). En sus controles de conexión y ajustes, verifica dirección del servicio, autenticación, proveedor, idioma y voz o perfil. Carga el catálogo antes de probar una frase breve sin datos sensibles. Investiga una voz ausente como posible problema de catálogo, propiedad o proveedor; no inventes su identificador.

En SonicBoom, la síntesis, la admisión a reproducción y la finalización audible son resultados distintos. Una respuesta en cola o iniciada no demuestra que alguien escuchara la frase. La voz clonada puede tardar más que la llegada de mensajes: revisa diagnósticos de cola y saturación y limita los eventos admitidos.

## Resultado esperado, estado y recuperación {#expected-result-status-and-recovery}

El audio local está disponible cuando el host anuncia su capacidad. La voz depende del plugin instalado y del servicio accesible; esta guía no garantiza aceptación de cada proveedor, dispositivo o plataforma. Si no suena, comprueba archivo, volumen, salida, pausa y error de ejecución. Para voz, verifica servicio y catálogo antes de repetir. Evita pruebas repetidas durante una emisión y consulta [diagnósticos](../preferences/troubleshooting.md) sin compartir tokens ni texto privado hablado.

## Operaciones de línea de comandos {#command-line-operations}

Cada operación de esta guía también está disponible desde la CLI: sin conexión, scriptable y apta para agentes. Las líneas de uso son sintaxis del parser y permanecen en inglés en todos los idiomas.

<!--@include: ../../../../.generated/cli-domain-media.es.md-->

Consulta la [referencia completa de comandos](../../developer/cli.md) para las convenciones, la salida JSON y los marcadores de operaciones destructivas.

<TourFrame id="automationActionMedia" locale="es" title="Guía interactiva: Acciones de overlay y sonido" />

<TourFrame id="automationActionTts" locale="es" title="Guía interactiva: Acciones de voz (TTS)" />
