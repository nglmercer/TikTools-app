---
title: "Ciclo de vida, propiedad y liberación"
docId: "developer.lifecycle"
translationKey: "developer.lifecycle"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3e866271067cf2c4d374aaccd9a6cdf6babf981df4deeff5313947cbf39b5cc7"
translationRevision: "4f39c691d3a10a546db414490086a0462ae5e15adf7f8b0be3bc99a01db9b88e"
order: 50
---

# Ciclo de vida, propiedad y liberación {#lifecycle-ownership-and-cleanup}

## Objetivo y requisitos {#goal-and-prerequisites}

Haz que montaje, reconexión y cierre liberen sus recursos. Conoce la [arquitectura](architecture.md) y utiliza un `TIKTOOLS_HOME` aislado para experimentar con el host nativo. Cancelar una suscripción frontend y cerrar el host resuelven responsabilidades distintas.

## Seguir la vida del escritorio {#follow-the-desktop-lifetime}

`crates/tiktools-desktop/src/app.rs` implementa el controlador de aplicación de Winit. Ventana y WebView se crean en el hilo del bucle de eventos. La ventana comienza oculta y aparece después de que Vue envíe `frontend-ready`. Cerrar la ventana la oculta cuando existe bandeja; sin ella inicia el cierre. Salir solicita al núcleo detener sus servicios antes de terminar el bucle.

`crates/tiktools-core/src/shutdown.rs` cierra pronto la admisión de nuevos plugins, detiene coordinación y monitorización, desconecta LIVE y drena el trabajo propio. Las llamadas repetidas esperan la misma finalización. Descartar una operación asíncrona no detiene código nativo arbitrario ni deshace efectos externos.

## Asignar los recursos frontend {#own-frontend-resources-explicitly}

Sigue el patrón de `use-app-runtime.ts`:

```ts
import { onUnmounted } from 'vue';
import { useControlClient } from '@/shared/api/use-control-client.ts';

const control = useControlClient();
const stop = control.onTopic('points.changed', () => {
  // Actualiza aquí la presentación confirmada del almacén propietario.
});
onUnmounted(stop);
```

El cuerpo de la función corresponde a la entidad propietaria de la vista. Una funcionalidad libera sus suscripciones, temporizadores, cuadros de animación y ámbitos de solicitud; el arranque de la aplicación conecta y desconecta el transporte compartido. Cambiar una plantilla de widget debe liberar el controlador anterior, incluidos sus temporizadores de alertas.

## Resultado esperado y verificación {#expected-result-and-verification}

Salir de una pantalla y volver no multiplica las llamadas. Ocultar la presentación evita renderizado innecesario sin pausar la contabilidad autoritativa. La reconexión informa huecos y carga instantáneas; no repite efectos de negocio. Ejecuta `bun scripts/task.ts test web cloud-lifecycle` y `bun scripts/task.ts test web request-scope` como ejemplos dirigidos.

## Recuperación y límites {#recovery-and-limits}

Un inicio de QuickJS agotado puede quedar incierto; el gestor conserva la identidad del hilo y bloquea su sustitución en vez de declarar un cierre exitoso. Algunas rutas síncronas de control de plugins todavía pueden bloquear al llamante. Considera estos límites al entregar una actualización. Detén limpiamente al propietario; no borres su bloqueo activo. El [código de cierre](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/shutdown.rs) y sus pruebas ofrecen evidencia más precisa que los diagramas históricos.

## Detalles de operación {#operational-details}

El cierre detiene admisión LIVE y drena ingreso, después cancela observadores/desconecta y detiene polling de plugins. Vacía Analytics, resuelve escrituras de puntos (30 s), hace checkpoint SQLite (5 s) y termina cierre nativo. Timeout de polling, fallo analítico, puntos pendientes o checkpoint fallido marcan insegura la actualización. `AppCore::update_shutdown_safe()` exige cierre terminado y bandera segura; terminar no basta. El updater consume ese resultado nativo: ningún interruptor del usuario certifica trabajo incierto. Conserva logs/recuperación pendiente y resuelve incertidumbre antes de sustituir binarios.
