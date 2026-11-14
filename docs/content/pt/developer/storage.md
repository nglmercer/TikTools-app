---
title: "Persistência e recuperação durável"
docId: "developer.storage"
translationKey: "developer.storage"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7ec9d94e484af17e8e82aaaa2122cc47ecd206cf9306047795d6aaa72eea2edb"
translationRevision: "ec562530a5513fb8a55704391bfc9741d0e19ab49ed4e16fedac4423eca1a95b"
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

## Detalhes de operação {#operational-details}

Confira caminhos antes de testes/cópias com `tiktools api call system.info --json`. `TIKTOOLS_HOME` escolhe a raiz; `TIKTOOLS_DATA_DIR`, `TIKTOOLS_PLUGINS_DIR`, `TIKTOOLS_PLUGIN_DATA_DIR`, `TIKTOOLS_PLUGIN_STORE_DIR`, `TIKTOOLS_LOG_DIR` e `TIKTOOLS_TEMP_DIR` substituem diretórios derivados. Valores relativos usam o diretório de inicialização. Uma raiz isolada não neutraliza overrides herdados: confira/remova-os antes.

Analytics contém agregados com escritor próprio: fila atual 1.024, lotes 128, prazo de flush 10 s. O arquivo vizinho `*.pending.json` deriva do caminho da base. A inicialização reaplica lotes identificados e remove o arquivo antes de `ready` admitir trabalho. Recuperação ilegível/acima de 16 MiB, base indisponível ou falha ao remover o arquivo mantêm admissão desativada. Preserve base e arquivo, inspecione `ready`/pendências/erros e faça backup antes do reparo. Não apague evidências para aparentar saúde; os diários de pontos têm outro contrato de durabilidade.

`system.info` informa home/dataDir efetivos; resolva outros diretórios pelo ambiente real de inicialização do host, não outro terminal. Use `system.health` para ready/pendências analíticas e contadores de entrada LIVE.
