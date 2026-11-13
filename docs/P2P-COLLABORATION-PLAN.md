# Peer transport migration plan / Plan de migración del transporte entre pares

This legacy URL is retained. Use the topic guides below for current instructions.

Se conserva esta dirección antigua. Usa las guías temáticas siguientes para las instrucciones actuales.

**Status / Estado:** historical design, release record or dated evidence; not current acceptance / diseño, registro de versión o evidencia histórica; no acredita aceptación actual.

**Source review:** Rust WebRTC code now lives in tiktools-rtc, while tiktools-peer still contains the earlier Iroh implementation. The historical Spanish plan must not imply Iroh removal, deployed backend signaling or completed physical acceptance.

**Revisión del código:** WebRTC en Rust vive en tiktools-rtc y tiktools-peer aún contiene Iroh. El plan histórico en español no debe interpretarse como retirada de Iroh, señalización ya desplegada o aceptación física completada.

<span id="1-prueba-rust-de-conectividad-y-decisión-de-dependencia"></span>
<span id="1-prueba-rust-de-conectividad-y-decisión-de-dependencia-"></span>
<span id="2-una-identidad-y-un-motor-de-permisos"></span>
<span id="2-una-identidad-y-un-motor-de-permisos-"></span>
<span id="3-primer-recorrido-completo-headless"></span>
<span id="3-primer-recorrido-completo-headless-"></span>
<span id="4-desktop-y-automatizaciones-existentes"></span>
<span id="4-desktop-y-automatizaciones-existentes-"></span>
<span id="5-despliegue-de-respaldo-y-observabilidad"></span>
<span id="5-despliegue-de-respaldo-y-observabilidad-"></span>
<span id="6-aceptación-física-y-live"></span>
<span id="6-aceptación-física-y-live-"></span>
<span id="_1-prueba-rust-de-conectividad-y-decision-de-dependencia"></span>
<span id="_2-una-identidad-y-un-motor-de-permisos"></span>
<span id="_3-primer-recorrido-completo-headless"></span>
<span id="_4-desktop-y-automatizaciones-existentes"></span>
<span id="_5-despliegue-de-respaldo-y-observabilidad"></span>
<span id="_6-aceptacion-fisica-y-live"></span>
<span id="adaptador-webrtc-rust-validacion-inicial"></span>
<span id="adaptador-webrtc-rust-validación-inicial"></span>
<span id="adaptador-webrtc-rust-validación-inicial-"></span>
<span id="alcance-para-mantenerlo-simple"></span>
<span id="alcance-para-mantenerlo-simple-"></span>
<span id="alternativas-revisadas"></span>
<span id="alternativas-revisadas-"></span>
<span id="arquitectura"></span>
<span id="arquitectura-"></span>
<span id="decision-vigente-sustituir-iroh"></span>
<span id="decisión-vigente-sustituir-iroh"></span>
<span id="decisión-vigente-sustituir-iroh-"></span>
<span id="descubrimiento-stun-y-base-cifrada-del-respaldo"></span>
<span id="descubrimiento-stun-y-base-cifrada-del-respaldo-"></span>
<span id="enrutamiento-del-respaldo-en-el-backend-existente"></span>
<span id="enrutamiento-del-respaldo-en-el-backend-existente-"></span>
<span id="evidencia-actual-y-trabajo-pendiente"></span>
<span id="evidencia-actual-y-trabajo-pendiente-"></span>
<span id="implementacion-anterior-y-evidencia-conservada"></span>
<span id="implementacion-por-entregas"></span>
<span id="implementación-anterior-y-evidencia-conservada"></span>
<span id="implementación-anterior-y-evidencia-conservada-"></span>
<span id="implementación-por-entregas"></span>
<span id="implementación-por-entregas-"></span>
<span id="objetivo-y-decision-propuesta"></span>
<span id="objetivo-y-decisión-propuesta"></span>
<span id="objetivo-y-decisión-propuesta-"></span>
<span id="plan-colaboracion-rust-con-webrtc-y-senalizacion-en-el-backend"></span>
<span id="plan-colaboración-rust-con-webrtc-y-señalización-en-el-backend"></span>
<span id="plan-colaboración-rust-con-webrtc-y-señalización-en-el-backend-"></span>
<span id="protocolo-inicial-acotado"></span>
<span id="protocolo-inicial-acotado-"></span>
<span id="que-significa-seguir-funcionando-si-cloud-cae"></span>
<span id="qué-significa-seguir-funcionando-si-cloud-cae"></span>
<span id="qué-significa-seguir-funcionando-si-cloud-cae-"></span>
<span id="reutilizar-el-contrato-de-colaboracion"></span>
<span id="reutilizar-el-contrato-de-colaboración"></span>
<span id="reutilizar-el-contrato-de-colaboración-"></span>
<span id="senalizacion-wss-implementada-en-remakebackend"></span>
<span id="señalización-wss-implementada-en-remakebackend"></span>
<span id="señalización-wss-implementada-en-remakebackend-"></span>

**Transports / Transportes** (Developer / Desarrollo): [English](content/en/developer/transports.md) · [Español](content/es/developer/transports.md).

**Collaboration contracts / Contratos de colaboración** (Developer / Desarrollo): [English](content/en/developer/collaboration.md) · [Español](content/es/developer/collaboration.md).

**Remote client / Cliente remoto** (Developer / Desarrollo): [English](content/en/developer/remote-client.md) · [Español](content/es/developer/remote-client.md).

**Exact historical source / Fuente histórica exacta:** [original Markdown](internal/archive/docs/P2P-COLLABORATION-PLAN.md).

Archived byte for byte from commit [`80c5cb1f`](https://github.com/nglmercer/TikTools-app/blob/80c5cb1fbfa45eaeb1c086c71320a15e565d8541/docs/P2P-COLLABORATION-PLAN.md); SHA-256 `d9814d74d44804b3c4cb816299645569921018a18eee13a2eb638c2c4d2b607c`.

Conservado byte por byte; los resultados fechados y las afirmaciones obsoletas se mantienen como antecedentes. Consulta [el mapa de migración](migration-map.json) para la auditoría y los destinos de cada sección.
