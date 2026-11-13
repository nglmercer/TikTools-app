---
title: "Contribuir con cambios acotados"
docId: "developer.contributing"
translationKey: "developer.contributing"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "164c90c98aae77782941bffba8a73c3991648b2f188705a62726321974b93e70"
translationRevision: "164c90c98aae77782941bffba8a73c3991648b2f188705a62726321974b93e70"
order: 230
---

# Contribuir con cambios acotados

## Objetivo y requisitos

Prepara un cambio que otra persona pueda revisar y verificar. Lee [CONTRIBUTING.md](https://github.com/nglmercer/TikTools-app/blob/remake/CONTRIBUTING.md), los `AGENTS.md` aplicables y [preparación](setup.md) antes de editar. Usa Bun 1.4.4, Rust 1.95 o posterior cuando corresponda, archivos de bloqueo existentes y una rama basada en el destino previsto.

## Definir e implementar el cambio

1. Inspecciona `git status` y conserva el trabajo ajeno. Excluye recursos generados, credenciales y bases locales del parche.
2. Sigue al propietario actual: núcleo Rust para servicios, módulos Control API para contratos, cliente tipado para cobertura SDK y [capas frontend](frontend.md) para presentación.
3. Ante cambios de contrato, modifica su definición canónica y regenera derivados juntos. No mantengas otra tabla completa manual de RPC o eventos.
4. Conserva las implementaciones particulares en paquetes externos de plugins; el host genérico descubre manifiestos en lugar de importar un plugin por nombre.
5. Explica el disparador concreto y el comportamiento resultante, incluidas rutas incompletas o experimentales.

## Validar según el alcance

```sh
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts contracts --check
```

Los cambios nativos también requieren formato, lint, pruebas Rust y puertas CI aplicables. Un cambio exclusivamente documental debe comprobar contenido, órdenes y enlaces, sin declarar aceptación del runtime nativo. El ejecutor del proyecto posee las entradas admitidas; usa el acceso Cargo para banderas específicas de destinos nativos.

## Resultado esperado y estado

La revisión contiene cambios concretos, evidencia útil y límites honestos. La política dirige solicitudes de cambio a `remake`; las etiquetas de versión las controla mantenimiento. Las instrucciones de contribución están disponibles, pero protección de ramas y comprobaciones remotas son responsabilidad administrativa, no garantías de una página Markdown.

## Recuperación

Si falla la validación por una dependencia o biblioteca de plataforma ausente, identifica y repara el entorno sin debilitar la comprobación. Si divergen archivos generados, regenera desde su propietario y revisa las diferencias. Nunca publiques cookies, claves, credenciales ni capturas sin sanear. Cuando código y diseño heredado discrepen, explica la diferencia y usa la implementación actual para orientación pública. No sobrescribas archivos pendientes de otra persona para aparentar coherencia en una limpieza general.
