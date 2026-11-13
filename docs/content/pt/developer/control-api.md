---
title: "Contratos e descoberta da API de controle"
docId: "developer.control-api"
translationKey: "developer.control-api"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "9ce931bc43614d39b264662b83fbdf3ddc7df875ef7cd6193fc6742eed34c509"
translationRevision: "add499299f3867d39f6f1887c94c5fa3444afb69e72887d0cc7390f0b4dd39ff"
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

## Detalhes de operação {#operational-details}

Use um `TIKTOOLS_HOME` descartável, remova overrides herdados conforme [armazenamento](storage.md), inicie `tiktools host --ipc` e use a mesma raiz no terminal de controle. A receita pressupõe o workflow descartável `docs-example`; crie antes o exemplo desativado de [automações](automations.md#operational-details). Preserve revisão de `meta.resourceVersions` e `meta.hostInstanceId` da leitura; substitua `REVISION`/`HOST_INSTANCE`. Mantenha um ID estável e seu payload exato. Após timeout consulte o ID original e atualize o recurso; `accepted`, `running`, `unknown` não autorizam outro efeito. Conflito exige nova leitura/revisão, não sobrescrita automática. `points.adjust` é aditivo: aceita ID de comando, sem revisão esperada.

`--include-meta` preserva resultado/metadados; a saída padrão extrai o resultado. `--dry-run --explain` valida/planeja sem executar; explicações vão a stderr e JSON a stdout. Operações destrutivas exigem `--confirm`. A saída oculta segredos reconhecidos; `--secrets-visible` só altera a renderização CLI e não recupera valores ocultados pelo host. Revise manualmente. `api events` limita tempo/contagem e filtra tópicos. Discovery do host determina features/permissões; descrições geradas preservam o texto exato do registro.

```sh
tiktools api discover --json
tiktools api schema workflows.disable --json
tiktools api call workflows.get '{"id":"docs-example"}' --include-meta --json
tiktools api call workflows.disable '{"id":"docs-example"}' --expected-revision REVISION --command-id docs-disable-001 --host-instance-id HOST_INSTANCE --include-meta --dry-run --explain --json
tiktools api call workflows.disable '{"id":"docs-example"}' --expected-revision REVISION --command-id docs-disable-001 --host-instance-id HOST_INSTANCE --include-meta --json
tiktools api call collaboration.commands.get '{"commandId":"docs-disable-001"}' --json
tiktools api call points.adjust '{"uniqueId":"docs-viewer","delta":1}' --command-id docs-points-001 --host-instance-id HOST_INSTANCE --include-meta --dry-run --json
tiktools api events --topics live.status --timeout 5 --max-events 2
```

```json
{"jsonrpc":"2.0","id":1,"method":"workflows.disable","params":{"id":"docs-example"},"meta":{"expectedRevision":"1","commandId":"docs-disable-001","hostInstanceId":"HOST_INSTANCE"}}
```

```json
{"jsonrpc":"2.0","id":1,"result":{"id":"docs-example","enabled":false},"meta":{"hostInstanceId":"HOST_INSTANCE","resourceVersions":[{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"}],"command":{"commandId":"docs-disable-001","state":"succeeded"}}}
```

```json
{"jsonrpc":"2.0","id":1,"error":{"code":"revision_conflict","message":"Resource revision changed","data":{"resourceVersions":[{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"}]}}}
```
