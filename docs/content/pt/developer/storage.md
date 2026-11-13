---
title: "Persistência e recuperação durável"
docId: "developer.storage"
translationKey: "developer.storage"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f6f86e9e87d6727f4dace206cef26a37d501f8030f7a1ff42688af77592a7445"
translationRevision: "f6f86e9e87d6727f4dace206cef26a37d501f8030f7a1ff42688af77592a7445"
order: 60
---

# Persistência e recuperação durável {#persistence-and-durable-recovery}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Altere estado persistente pelo serviço responsável e preserve resultados recuperáveis. Use um perfil descartável, feche seu host antes de copiar bancos de dados e leia [metadados de controle](control-api.md) antes de editar recursos versionados. `TIKTOOLS_HOME` seleciona a raiz do aplicativo; `AppPaths` resolve seus dados e armazenamentos privados.

## Identifique o armazenamento {#identify-the-store}

`crates/tiktools-core/src/paths.rs` define `data/tiktok-points.db`, `data/tiktok-automation.db` e `data/tiktok-analytics.db`. Os módulos de serviço SQLite são responsáveis por seus esquemas. Pontos/espectadores e definições de automação têm registros de negócio separados; análises usam um banco distinto, omitido em diagramas antigos de dois bancos.

A inicialização pode copiar bancos legados locais do checkout para os dados de aplicativo da plataforma somente quando o destino estiver ausente. Ela nunca sobrescreve um destino existente. Definições JSON continuam legíveis durante alterações na execução nativa. Não implemente migração excluindo o perfil do usuário nem reescrevendo tabelas a partir do Vue.

## Faça uma alteração recuperável {#make-a-recoverable-change}

1. Localize a transação do serviço e seus testes de esquema/migração.
2. Armazene a alteração de negócio, revisão e reserva obrigatória de diário/outbox na mesma transação quando o domínio permitir.
3. Preserve marcadores de exclusão de revisão. Um ID de registro reutilizado não deve aceitar uma revisão de edição anterior à exclusão.
4. Publique metadados de invalidação confirmados pela outbox existente; uma nova tentativa de entrega não deve repetir a ação subjacente.
5. Reabra o mesmo diretório privado de teste após uma interrupção e resolva o comando original antes de tentar outra mutação.

Verificações úteis somente de leitura em um host de teste já em execução:

```sh
target/debug/tiktools --json system info
target/debug/tiktools --json system doctor
target/debug/tiktools api discover
```

## Resultado esperado e status {#expected-result-and-status}

Diários duráveis de fluxos, pontos e proprietário compatíveis mantêm os resultados originais após reiniciar. Um recibo descreve seu comando histórico; leia o estado atual separadamente. Armazenamentos de workspace do Cloud adicionam partições de origem/conta e verificações de geração; não substituem a autoridade SQLite local. Nem todo efeito externo ou domínio tem a mesma cobertura de recuperação durável.

## Recuperação {#recovery}

Mantenha arquivos SQLite e quaisquer WAL/SHM ativos juntos ao fazer um backup offline. Um tempo limite da interface não comprova reversão. Em `revision_conflict`, preserve o rascunho e compare uma leitura nova. Para efeitos externos incertos, inspecione o registro de efeito mantido em vez de reproduzir um feed. Consulte a [autoridade de caminhos](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/paths.rs) e a [recuperação do Cloud](cloud.md) para seus limites distintos.
