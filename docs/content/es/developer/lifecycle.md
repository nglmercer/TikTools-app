---
title: "Ciclo de vida, propiedad y liberación"
docId: "developer.lifecycle"
translationKey: "developer.lifecycle"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "96d910c8c477184ce9b7346953063ede759327e97855cb72a9ae0b80732d4f56"
translationRevision: "96d910c8c477184ce9b7346953063ede759327e97855cb72a9ae0b80732d4f56"
order: 50
---

# Ciclo de vida, propiedad y liberación

## Objetivo y requisitos

Haz que montaje, reconexión y cierre liberen sus recursos. Conoce la [arquitectura](architecture.md) y utiliza un `TIKTOOLS_HOME` aislado para experimentar con el host nativo. Cancelar una suscripción frontend y cerrar el host resuelven responsabilidades distintas.

## Seguir la vida del escritorio

`crates/tiktools-desktop/src/app.rs` implementa el controlador de aplicación de Winit. Ventana y WebView se crean en el hilo del bucle de eventos. La ventana comienza oculta y aparece después de que Vue envíe `frontend-ready`. Cerrar la ventana la oculta cuando existe bandeja; sin ella inicia el cierre. Salir solicita al núcleo detener sus servicios antes de terminar el bucle.

`crates/tiktools-core/src/shutdown.rs` cierra pronto la admisión de nuevos plugins, detiene coordinación y monitorización, desconecta LIVE y drena el trabajo propio. Las llamadas repetidas esperan la misma finalización. Descartar una operación asíncrona no detiene código nativo arbitrario ni deshace efectos externos.

## Asignar los recursos frontend

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

## Resultado esperado y verificación

Salir de una pantalla y volver no multiplica las llamadas. Ocultar la presentación evita renderizado innecesario sin pausar la contabilidad autoritativa. La reconexión informa huecos y carga instantáneas; no repite efectos de negocio. Ejecuta `bun scripts/task.ts test web cloud-lifecycle` y `bun scripts/task.ts test web request-scope` como ejemplos dirigidos.

## Recuperación y límites

Un inicio de QuickJS agotado puede quedar incierto; el gestor conserva la identidad del hilo y bloquea su sustitución en vez de declarar un cierre exitoso. Algunas rutas síncronas de control de plugins todavía pueden bloquear al llamante. Considera estos límites al entregar una actualización. Detén limpiamente al propietario; no borres su bloqueo activo. El [código de cierre](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/shutdown.rs) y sus pruebas ofrecen evidencia más precisa que los diagramas históricos.
