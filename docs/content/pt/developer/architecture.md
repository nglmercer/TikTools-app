---
title: "Arquitetura do aplicativo e dos serviços"
docId: "developer.architecture"
translationKey: "developer.architecture"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "9726c146c879432a9964d4eaed6e35b3645100b390f6e08e87b67b2a4e854bb8"
translationRevision: "9726c146c879432a9964d4eaed6e35b3645100b390f6e08e87b67b2a4e854bb8"
order: 30
---

# Arquitetura do aplicativo e dos serviços {#application-and-service-architecture}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Coloque uma alteração no serviço responsável por seu estado. Leia a [configuração](setup.md) e entenda a diferença entre um host existente e um host explicitamente independente. O desktop constrói um `AppCore`, envolve-o em um `ControlApi` e expõe essa instância à WebView, CLI local e agentes.

## Rastreie uma operação {#trace-an-operation}

1. Encontre o registro do método em `crates/tiktools-control-api/src/modules/`. O registro fornece esquemas, políticas e metadados de descoberta.
2. Siga o manipulador até `crates/tiktools-core/src/control/` e o serviço responsável. Coloque a validação de domínio e as mutações persistentes aqui, em vez de um adaptador de transporte.
3. Adicione o wrapper tipado em `crates/tiktools-client`; seu teste de paridade compara a cobertura do SDK com a superfície registrada.
4. Consuma o método por `src/web/shared/api/control-client.ts`. Entidades Vue mantêm estado de apresentação; o desktop não entrega à WebView um banco de dados nem um objeto de integração nativa.

Winit é responsável pelo laço de eventos, Wry pela WebView e a integração da bandeja pela bandeja. `HostEmitter` e eventos de domínio levam valores serializados para fora do núcleo. O desktop encaminha trabalho por seu proxy do laço de eventos; a avaliação de scripts da WebView permanece nessa thread. Tokio executa operações assíncronas do host.

## Verificação concreta dos limites {#concrete-boundary-check}

```sh
bun scripts/task.ts test web control-client
bun scripts/task.ts rust test -p tiktools-client --test parity --locked
```

O primeiro comando verifica o cliente de frontend. O segundo executa diretamente o alvo de integração do SDK preservando o lockfile; flags de alvo do Cargo pertencem ao repasse `rust`, em vez da gramática mais restrita de testes específicos.

## Resultado esperado e status {#expected-result-and-status}

Todos os clientes observam a mesma operação autoritativa e o mesmo caminho de eventos. `host --ipc`, `host --stdio` e `--standalone` criam intencionalmente propriedade sem interface gráfica; comandos CLI normais se conectam a um proprietário em execução. O caminho legado `PageMessage`/`IpcRouter` permanece como código de compatibilidade e não deve receber novas variantes de mensagens.

## Recuperação {#recovery}

Se dois clientes divergirem, compare identidade do host, diretório de dados, recursos de compilação e descoberta antes de adicionar outro store. Um plugin ausente é um problema de catálogo de execução, não um motivo para compilar sua implementação no núcleo. Documentos históricos de arquitetura contêm caminhos de frontend aposentados; siga o [núcleo atual](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/lib.rs) e o [mapa do frontend](frontend.md) quando divergirem.

![AppCore do TikTools, transporte, ControlClient e workspace Vue real](/architecture.svg)
