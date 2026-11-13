# Retired napi-vm engine migration plan / Plan retirado de migración del motor napi-vm

This legacy URL is retained. Use the topic guides below for current instructions.

Se conserva esta dirección antigua. Usa las guías temáticas siguientes para las instrucciones actuales.

**Status / Estado:** historical design, release record or dated evidence; not current acceptance / diseño, registro de versión o evidencia histórica; no acredita aceptación actual.

**Source review:** The dependency migration proposed here is obsolete: Cargo.toml and the script service now use rquickjs. Only the serialized runtime name napi-vm survives; the old upstream VM revisions are not installation instructions.

**Revisión del código:** La migración propuesta quedó obsoleta: Cargo.toml y el servicio de scripts usan rquickjs. Solo se conserva el nombre serializado napi-vm; las revisiones del motor anterior no son instrucciones de instalación.

<span id="acceptance-gates"></span>
<span id="acceptance-gates-"></span>
<span id="baseline-and-target"></span>
<span id="baseline-and-target-"></span>
<span id="implementation-sequence"></span>
<span id="implementation-sequence-"></span>
<span id="napi-vm-migration-plan"></span>
<span id="napi-vm-migration-plan-"></span>
<span id="rollout-and-rollback"></span>
<span id="rollout-and-rollback-"></span>

**Plugin runtimes / Runtimes de plugins** (Developer / Desarrollo): [English](content/en/developer/plugins/runtimes.md) · [Español](content/es/developer/plugins/runtimes.md).

**Architecture / Arquitectura** (Developer / Desarrollo): [English](content/en/developer/architecture.md) · [Español](content/es/developer/architecture.md).

**Exact historical source / Fuente histórica exacta:** [original Markdown](internal/archive/docs/NAPI-VM-MIGRATION-PLAN.md).

Archived byte for byte from commit [`80c5cb1f`](https://github.com/nglmercer/TikTools-app/blob/80c5cb1fbfa45eaeb1c086c71320a15e565d8541/docs/NAPI-VM-MIGRATION-PLAN.md); SHA-256 `ec7df3b7da6cdfb9f0e07eb776cdf3a9f2b15c7562fd705decca28c0f39a7ebd`.

Conservado byte por byte; los resultados fechados y las afirmaciones obsoletas se mantienen como antecedentes. Consulta [el mapa de migración](migration-map.json) para la auditoría y los destinos de cada sección.
