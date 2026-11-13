# Reliability refactor progress record / Registro del refactor de fiabilidad

This legacy URL is retained. Use the topic guides below for current instructions.

Se conserva esta dirección antigua. Usa las guías temáticas siguientes para las instrucciones actuales.

**Status / Estado:** historical design, release record or dated evidence; not current acceptance / diseño, registro de versión o evidencia histórica; no acredita aceptación actual.

**Source review:** The current ingress, points writer and plugin worker retain bounded admission and lifecycle accounting. The baseline-specific progress table and earlier unresolved gates remain dated evidence, not a current issue tracker or acceptance certificate.

**Revisión del código:** El ingreso, escritor de puntos y worker de plugins conservan admisión limitada y contabilidad del ciclo de vida. La tabla del refactor y sus requisitos pendientes son evidencia fechada, no el seguimiento actual ni un certificado de aceptación.

<span id="additional-frontend-regressions"></span>
<span id="additional-frontend-regressions-"></span>
<span id="batch-1-transport-ownership-and-ingress-generation-accounting"></span>
<span id="batch-1-transport-ownership-and-ingress-generationaccounting"></span>
<span id="batch-1-transport-ownership-and-ingress-generationaccounting-"></span>
<span id="batch-2-fan-out-and-polling-isolation"></span>
<span id="batch-2-fan-out-and-polling-isolation-"></span>
<span id="batch-3-frontend-admission-and-disposal"></span>
<span id="batch-3-frontend-admission-and-disposal-"></span>
<span id="batch-4-connection-task-failure-guard"></span>
<span id="batch-4-connection-task-failure-guard-"></span>
<span id="batch-6-plugin-queue-accounting"></span>
<span id="batch-6-plugin-queue-accounting-"></span>
<span id="batch-7-measurement-and-verification-record"></span>
<span id="batch-7-measurement-and-verification-record-"></span>
<span id="completed-batches"></span>
<span id="completed-batches-"></span>
<span id="current-implementation-audit"></span>
<span id="current-implementation-audit-"></span>
<span id="evidence-and-validation"></span>
<span id="evidence-and-validation-"></span>
<span id="final-rust-rerun-on-the-implemented-batches"></span>
<span id="final-rust-rerun-on-the-implemented-batches-"></span>
<span id="live-reliability-refactor-progress"></span>
<span id="live-reliability-refactor-progress-"></span>
<span id="october-2026-desktop-transport-follow-up"></span>
<span id="october-2026-desktop-transport-follow-up-"></span>
<span id="phase-1--loss-aware-graceful-ingress-shutdown-prompt-phase-1"></span>
<span id="phase-1--loss-aware-graceful-ingress-shutdown-prompt-phase-1-"></span>
<span id="phase-1-—-loss-aware-graceful-ingress-shutdown-prompt-phase-1"></span>
<span id="reviewable-commit-sequence"></span>
<span id="reviewable-commit-sequence-"></span>

**Performance / Rendimiento** (Developer / Desarrollo): [English](content/en/developer/performance.md) · [Español](content/es/developer/performance.md).

<span id="batch-5-owned-bounded-live-points-execution"></span>
<span id="batch-5-owned-bounded-live-points-execution-"></span>
<span id="phase-2--durable-points-fifo-ordering-and-sqlite-correctness"></span>
<span id="phase-2--durable-points-fifo-ordering-and-sqlite-correctness-"></span>
<span id="phase-2-—-durable-points-fifo-ordering-and-sqlite-correctness"></span>

**Event delivery / Entrega de eventos** (Developer / Desarrollo): [English](content/en/developer/events.md) · [Español](content/es/developer/events.md).

<span id="phase-3--plugin-worker-lifecycle-and-isolation"></span>
<span id="phase-3--plugin-worker-lifecycle-and-isolation-"></span>
<span id="phase-3-—-plugin-worker-lifecycle-and-isolation"></span>

**Desktop lifecycle / Ciclo de vida del escritorio** (Developer / Desarrollo): [English](content/en/developer/lifecycle.md) · [Español](content/es/developer/lifecycle.md).

**Exact historical source / Fuente histórica exacta:** [original Markdown](internal/archive/docs/PERFORMANCE_REFACTOR_PROGRESS.md).

Archived byte for byte from commit [`80c5cb1f`](https://github.com/nglmercer/TikTools-app/blob/80c5cb1fbfa45eaeb1c086c71320a15e565d8541/docs/PERFORMANCE_REFACTOR_PROGRESS.md); SHA-256 `ff59098ba2322dc5b313f7350838e289fe086fb7bb1104dba4ff72986aa3dc98`.

Conservado byte por byte; los resultados fechados y las afirmaciones obsoletas se mantienen como antecedentes. Consulta [el mapa de migración](migration-map.json) para la auditoría y los destinos de cada sección.
