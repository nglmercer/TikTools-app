# WebView presentation contract / Contrato de presentación WebView

This legacy URL is retained. Use the topic guides below for current instructions.

Se conserva esta dirección antigua. Usa las guías temáticas siguientes para las instrucciones actuales.

**Status / Estado:** experimental or incomplete; verify the limits in the current guide / experimental o incompleto; revisa los límites de la guía actual.

**Source review:** ui.projection remains a desktop IPC concern, with page revisions and a 512-event render journal. The shared control client accepts injected transports; browser clients must use their own backfill path rather than emulate desktop projection.

**Revisión del código:** ui.projection sigue siendo una operación del IPC de escritorio con revisiones de página y un diario de 512 eventos. El cliente admite transportes inyectados; el navegador debe usar su recuperación propia y no emular la proyección.

<span id="frame-scheduling"></span>
<span id="frame-scheduling-"></span>
<span id="outstanding-gates"></span>
<span id="outstanding-gates-"></span>
<span id="pinned-rendering-and-paused-scrolling"></span>
<span id="pinned-rendering-and-paused-scrolling-"></span>
<span id="renderer-filter-a-b"></span>
<span id="renderer-filter-ab"></span>
<span id="renderer-filter-ab-"></span>
<span id="reproducible-native-workload"></span>
<span id="reproducible-native-workload-"></span>

**Desktop lifecycle / Ciclo de vida del escritorio** (Developer / Desarrollo): [English](content/en/developer/lifecycle.md) · [Español](content/es/developer/lifecycle.md).

**Event delivery / Entrega de eventos** (Developer / Desarrollo): [English](content/en/developer/events.md) · [Español](content/es/developer/events.md).

**Performance / Rendimiento** (Developer / Desarrollo): [English](content/en/developer/performance.md) · [Español](content/es/developer/performance.md).

<span id="latest-native-lifecycle-evidence"></span>
<span id="latest-native-lifecycle-evidence-"></span>
<span id="main-webview-presentation-contract-and-validation"></span>
<span id="main-webview-presentation-contract-and-validation-"></span>

**Testing / Pruebas** (Developer / Desarrollo): [English](content/en/developer/testing.md) · [Español](content/es/developer/testing.md).

**Exact historical source / Fuente histórica exacta:** [original Markdown](internal/archive/docs/LIVE_WEBVIEW_PROJECTION.md).

Archived byte for byte from commit [`80c5cb1f`](https://github.com/nglmercer/TikTools-app/blob/80c5cb1fbfa45eaeb1c086c71320a15e565d8541/docs/LIVE_WEBVIEW_PROJECTION.md); SHA-256 `5bf8a51744eb952bebbf840142e68ffa2d111fbb3cbc5cd3424b18de4589692d`.

Conservado byte por byte; los resultados fechados y las afirmaciones obsoletas se mantienen como antecedentes. Consulta [el mapa de migración](migration-map.json) para la auditoría y los destinos de cada sección.
