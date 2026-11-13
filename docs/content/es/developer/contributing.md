---
title: "Contribuir con cambios acotados"
docId: "developer.contributing"
translationKey: "developer.contributing"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "2e0522178ebee69ea5eb20fec3c0ec586f13950c51d658cb3a21843536b2f9f9"
translationRevision: "101d8387f6ca3a6452434775ed0a609eb7bc68615a8ccd73c121df74e300fc0c"
order: 230
---

# Contribuir con cambios acotados {#contributing-focused-changes}

## Objetivo y requisitos {#goal-and-prerequisites}

Prepara un cambio que otra persona pueda revisar y verificar. Lee [CONTRIBUTING.md](https://github.com/nglmercer/TikTools-app/blob/remake/CONTRIBUTING.md), los `AGENTS.md` aplicables y [preparación](setup.md) antes de editar. Usa Bun 1.4.4, Rust 1.95 o posterior cuando corresponda, archivos de bloqueo existentes y una rama basada en el destino previsto.

## Definir e implementar el cambio {#define-and-implement-the-change}

1. Inspecciona `git status` y conserva el trabajo ajeno. Excluye recursos generados, credenciales y bases locales del parche.
2. Sigue al propietario actual: núcleo Rust para servicios, módulos Control API para contratos, cliente tipado para cobertura SDK y [capas frontend](frontend.md) para presentación.
3. Ante cambios de contrato, modifica su definición canónica y regenera derivados juntos. No mantengas otra tabla completa manual de RPC o eventos.
4. Conserva las implementaciones particulares en paquetes externos de plugins; el host genérico descubre manifiestos en lugar de importar un plugin por nombre.
5. Explica el disparador concreto y el comportamiento resultante, incluidas rutas incompletas o experimentales.

## Validar según el alcance {#validate-proportionately}

```sh
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts contracts --check
```

Los cambios nativos también requieren formato, lint, pruebas Rust y puertas CI aplicables. Un cambio exclusivamente documental debe comprobar contenido, órdenes y enlaces, sin declarar aceptación del runtime nativo. El ejecutor del proyecto posee las entradas admitidas; usa el acceso Cargo para banderas específicas de destinos nativos.

## Resultado esperado y estado {#expected-result-and-status}

La revisión contiene cambios concretos, evidencia útil y límites honestos. La política dirige solicitudes de cambio a `remake`; las etiquetas de versión las controla mantenimiento. Las instrucciones de contribución están disponibles, pero protección de ramas y comprobaciones remotas son responsabilidad administrativa, no garantías de una página Markdown.

## Recuperación {#recover}

Si falla la validación por una dependencia o biblioteca de plataforma ausente, identifica y repara el entorno sin debilitar la comprobación. Si divergen archivos generados, regenera desde su propietario y revisa las diferencias. Nunca publiques cookies, claves, credenciales ni capturas sin sanear. Cuando código y diseño heredado discrepen, explica la diferencia y usa la implementación actual para orientación pública. No sobrescribas archivos pendientes de otra persona para aparentar coherencia en una limpieza general.

## Detalles de operación {#operational-details}

Los sellos tienen significados separados: `sourceRevision` resume cuerpo inglés actual y `translationRevision` el cuerpo localizado de este artículo (sin frontmatter, sin blancos externos). Tras revisar fuente y traducciones usa `bun docs/scripts/record-review.ts developer/example.md`. Un cambio posterior invalida el sello correspondiente. Detecta cambios, no identidad humana ni equivalencia semántica: compara comandos, IDs RPC/entorno, claves payload, límites, activación/recuperación y negaciones. Traduce prosa completa conservando instrucciones técnicas.

Para migración dividida registra tema/encabezado original y destino concreto en mapa; usa fragmento si apunta a sección. El checker valida archivos/fragmentos y bytes históricos, pero revisión editorial verifica cobertura temática. No edites originales/provenance históricos para aprobar.
