# QuickJS plugins with the napi-vm manifest name / Plugins QuickJS con el nombre de manifiesto napi-vm

This legacy URL is retained. Use the topic guides below for current instructions.

Se conserva esta dirección antigua. Usa las guías temáticas siguientes para las instrucciones actuales.

**Status / Estado:** implemented behavior mixed with experimental or historical material / comportamiento implementado mezclado con material experimental o histórico.

**Source review:** Manifest runtime napi-vm is retained for compatibility, backed by QuickJS and the in-process Node-API shim. It is not a full Node.js/libuv environment; trusted native addons cross the JavaScript sandbox boundary.

**Revisión del código:** El runtime napi-vm se conserva por compatibilidad sobre QuickJS y el puente Node-API dentro del proceso. No es un entorno completo Node.js/libuv; los addons nativos de confianza superan el aislamiento de JavaScript.

<span id="authorization"></span>
<span id="authorization-"></span>
<span id="declarative-native-libraries-nativelibs"></span>
<span id="declarative-native-libraries-nativelibs-"></span>
<span id="event-loop-and-shutdown"></span>
<span id="event-loop-and-shutdown-"></span>
<span id="follow-ups"></span>
<span id="follow-ups-"></span>
<span id="guest-environment"></span>
<span id="guest-environment-"></span>
<span id="guest-interface"></span>
<span id="guest-interface-"></span>
<span id="package-layout"></span>
<span id="package-layout-"></span>
<span id="packaging"></span>
<span id="packaging-"></span>
<span id="pending-bounded-startup-delivery-and-lifecycle-admission"></span>
<span id="pending-bounded-startup-delivery-and-lifecycle-admission-"></span>
<span id="real-world-example"></span>
<span id="real-world-example-"></span>
<span id="reference-fixture"></span>
<span id="reference-fixture-"></span>
<span id="rust-runtime-api-migration"></span>
<span id="rust-runtime-api-migration-"></span>
<span id="staging-native-packages"></span>
<span id="staging-native-packages-"></span>
<span id="threading-and-security"></span>
<span id="threading-and-security-"></span>

**Plugin runtimes / Runtimes de plugins** (Developer / Desarrollo): [English](content/en/developer/plugins/runtimes.md) · [Español](content/es/developer/plugins/runtimes.md).

**Author a plugin / Crear un plugin** (Developer / Desarrollo): [English](content/en/developer/plugins/quickstart.md) · [Español](content/es/developer/plugins/quickstart.md).

<span id="bundled-native-addons-napi-rs-node"></span>
<span id="bundled-native-addons-napi-rs-node-"></span>
<span id="manifest"></span>
<span id="manifest-"></span>
<span id="minimal-manifest"></span>
<span id="minimal-manifest-"></span>
<span id="napi-vm-plugins-runtime-napi-vm"></span>
<span id="napi-vm-plugins-runtime-napi-vm-"></span>

**Package and registry protocol / Protocolo de paquetes y registro** (Developer / Desarrollo): [English](content/en/developer/plugins/packaging-registry.md) · [Español](content/es/developer/plugins/packaging-registry.md).

**Exact historical source / Fuente histórica exacta:** [original Markdown](internal/archive/docs/NAPI_VM_PLUGINS.md).

Archived byte for byte from commit [`80c5cb1f`](https://github.com/nglmercer/TikTools-app/blob/80c5cb1fbfa45eaeb1c086c71320a15e565d8541/docs/NAPI_VM_PLUGINS.md); SHA-256 `007ac1969246e54516fe0afdcde2926df11f986459d2e30a7e8566c6f0e3a5bf`.

Conservado byte por byte; los resultados fechados y las afirmaciones obsoletas se mantienen como antecedentes. Consulta [el mapa de migración](migration-map.json) para la auditoría y los destinos de cada sección.
