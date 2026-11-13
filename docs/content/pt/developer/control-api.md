---
title: "Contratos e descoberta da API de controle"
docId: "developer.control-api"
translationKey: "developer.control-api"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "36ce27660edc95aea83a48cd408dbc683affaeefd2c686ee7229aeecc22d1ac8"
translationRevision: "36ce27660edc95aea83a48cd408dbc683affaeefd2c686ee7229aeecc22d1ac8"
order: 70
---

# Contratos e descoberta da API de controle {#control-api-contracts-and-discovery}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Chame o contrato publicado pelo host sem inventar nomes de métodos ou esquemas de resposta. Use um host privado em execução e a CLI correspondente. O [registro Rust](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-control-api/src/registry.rs) canônico descreve métodos; os módulos de registro fornecem seus esquemas, capacidades, requisitos de desktop e flags de concorrência. Descubra a compilação real antes de escolher uma operação.

## Inspecione e depois chame {#inspect-then-call}

```sh
target/debug/tiktools api discover
target/debug/tiktools api schema system.info
target/debug/tiktools api call system.info '{}' --format json
target/debug/tiktools api verify --sandbox
```

O último comando usa um ambiente de verificação isolado. Ele não valida uma conexão real com LIVE. Em stdio/IPC local, o formato de transmissão é JSON delimitado por linhas; WSS remoto carrega uma mensagem por quadro e exige primeiro `collaboration.hello`.

```json
{"jsonrpc":"2.0","id":"info-1","method":"system.info","params":{}}
```

Uma resposta repete o ID e contém `result` ou `error`. Erros usam um `code` textual, uma mensagem e `data` seguros opcionais; não presuma códigos numéricos de erro de um servidor JSON-RPC não relacionado. Notificações usam `method:"event"` e `params:{topic,data}`. Mantenha notificações separadas da resolução de respostas pendentes.

## Preserve metadados {#preserve-metadata}

Chamadas versionadas podem conter `meta.expectedRevision` no nível superior. Chamadas duráveis também vinculam um ID de comando e uma instância do host conforme a descoberta. Leia `meta.resourceVersions`, `meta.command` e `meta.hostInstanceId` junto do resultado de domínio. Revisões são strings decimais, não números JavaScript. O auxiliar `callWithMeta` do frontend em `src/web/shared/api/control-contracts.ts` preserva esse envelope.

## Resultado esperado e status {#expected-result-and-status}

Um método descoberto é compatível com aquela compilação do host; permissões remotas e restrições de recursos ainda se aplicam. Métodos do proprietário local não são implicitamente métodos remotos. Requisições genéricas mantêm o mesmo caminho de operação dos wrappers tipados do SDK. Referências técnicas compartilhadas devem derivar das fontes de registro/esquema em vez de uma segunda tabela de métodos mantida manualmente.

## Recuperação {#recovery}

Em `method_not_found`, compare descoberta e versões de cliente/host. Em `revision_conflict`, mantenha a candidata e compare as versões atuais. Um tempo limite de transporte pode deixar uma mutação aceita em execução; consulte seu ID original de comando mantido quando compatível. Nunca crie um ID novo apenas para contornar a incerteza. Continue com [agentes](agents.md) ou [recuperação remota](remote-client.md).

## Referência versionada de métodos {#method-reference}

Gerada a partir do instantâneo de descoberta versionado; o host em execução continua sendo a autoridade.

<!--@include: ../../../.generated/control-api.pt.md-->
