---
title: "Referência de comandos CLI"
docId: "developer.cli"
translationKey: "developer.cli"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6bb485d6e8067cb1639fb20821c45b79941245692f0b6c7525f2b3bee930048b"
translationRevision: "5922b8bff2a025a8a5f3b9255d27787f5de6e367a3026e016c6f2db98e9af959"
order: 75
---

# Referência de comandos CLI {#cli-command-reference}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Descubra cada forma de comando `tiktools` sem ler o código Rust nem analisar a saída de ajuda humana. Tenha um binário `tiktools` correspondente compilado; a referência é offline e nunca precisa de um host ativo. A fonte canônica é o registro de especificações em `crates/tiktools-cli/src/commands/spec.rs`; cada módulo de domínio possui as cadeias de uso que seu parser reporta em erros, portanto esta referência não pode divergir do que o parser aceita.

## Inspecionar a ajuda offline {#inspect-help-offline}

A ajuda humana agrupa os verbos por domínio; o filtro mostra uma seção de domínio. A forma `--json` imprime o registro legível por máquina que todo agente deveria preferir: uso, resumo e marcador destrutivo por verbo, mais as convenções globais da CLI, versionado pelo campo `schema` superior.

```sh
tiktools help
tiktools help plugin
tiktools help --json
```

Erros de uso imprimem apenas a forma relevante com uma dica em vez da página completa, então um erro de digitação nunca rola o terminal. O código de saída 2 sempre significa erro de uso; consulte o rodapé de convenções para as regras de resolução do host, saída e redação de segredos.

## Referência de comandos gerada {#generated-command-reference}

Gerada a partir do instantâneo de ajuda CLI versionado em `docs/contracts/cli-help.json`; o binário continua sendo a autoridade. As linhas de uso são sintaxe do parser e permanecem em inglês em todas as localidades; os resumos e convenções vêm dos catálogos de tradução da própria CLI, portanto esta referência não pode divergir do que `tiktools help` imprime no mesmo idioma. Os verbos marcados como destrutivos mutam ou destroem estado; scripts deveriam confirmar antes de executá-los.

<!--@include: ../../../.generated/cli.pt.md-->

## Detalhes operacionais {#operational-details}

O instantâneo regenera a partir de um binário compilado com `--features local-web` para que os domínios condicionados por features (cloud, publisher, web) estejam completos. A verificação de proveniência em `docs check` compara hashes de cada fonte Rust, então qualquer mudança nas especificações de comandos falha a compilação até regenerar o contrato. As compilações normais de documentação nunca executam Cargo.

```sh
bun docs/scripts/regenerate-control.ts
bun docs/scripts/generate-references.ts --check
bun docs/scripts/cli-coverage.ts
```

Execute a regeneração após alterar qualquer parser ou especificação de comando e revise o diff do fragmento como parte da mudança. Os agentes que consomem `help --json` deveriam tratar a versão 1 do esquema como estável de forma aditiva: novos verbos, domínios ou campos podem aparecer sem aviso; as chaves existentes mantêm seu significado.
