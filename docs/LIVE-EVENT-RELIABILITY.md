# LIVE event reliability / Fiabilidad de eventos LIVE

This legacy URL is retained. Use the topic guides below for current instructions.

Se conserva esta dirección antigua. Usa las guías temáticas siguientes para las instrucciones actuales.

**Status / Estado:** implemented behavior mixed with experimental or historical material / comportamiento implementado mezclado con material experimental o histórico.

**Source review:** Ingress remains bounded at 2,048 items; replay dedup uses 4,096 entries and a 300-second TTL. These are overload policies, not lossless delivery guarantees, and the current signer pin is defined in Cargo.toml.

**Revisión del código:** El ingreso sigue limitado a 2.048 elementos; la deduplicación usa 4.096 entradas y 300 segundos. Son políticas de sobrecarga, no garantías de entrega sin pérdidas; Cargo.toml define el signer vigente.

<span id="connection-retry-policy"></span>
<span id="connection-retry-policy-"></span>
<span id="delivery-accounting-and-recovery"></span>
<span id="delivery-accounting-and-recovery-"></span>
<span id="live-event-reliability"></span>
<span id="live-event-reliability-"></span>
<span id="upstream-decoder-migration"></span>
<span id="upstream-decoder-migration-"></span>

**Event delivery / Entrega de eventos** (Developer / Desarrollo): [English](content/en/developer/events.md) · [Español](content/es/developer/events.md).

**Performance / Rendimiento** (Developer / Desarrollo): [English](content/en/developer/performance.md) · [Español](content/es/developer/performance.md).

<span id="validation-and-limits"></span>
<span id="validation-and-limits-"></span>

**Testing / Pruebas** (Developer / Desarrollo): [English](content/en/developer/testing.md) · [Español](content/es/developer/testing.md).

**Exact historical source / Fuente histórica exacta:** [original Markdown](internal/archive/docs/LIVE-EVENT-RELIABILITY.md).

Archived byte for byte from commit [`80c5cb1f`](https://github.com/nglmercer/TikTools-app/blob/80c5cb1fbfa45eaeb1c086c71320a15e565d8541/docs/LIVE-EVENT-RELIABILITY.md); SHA-256 `ce1c87fa6134b636e1006d0a9ac82a62b78d772bceaf5bc994886f791411ee02`.

Conservado byte por byte; los resultados fechados y las afirmaciones obsoletas se mantienen como antecedentes. Consulta [el mapa de migración](migration-map.json) para la auditoría y los destinos de cada sección.
