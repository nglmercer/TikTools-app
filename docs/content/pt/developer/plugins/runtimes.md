---
title: "Ambientes de execução de plugins e limites de confiança"
docId: "developer.plugins.runtimes"
translationKey: "developer.plugins.runtimes"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6a249fa36fa53f3bfe7da57fdd25a445e593517946f80ac7e891b54d95cdc17d"
translationRevision: "1b8cf741b008d38404fbf91d5c8b0295f0628f0b2e68c477ee94761e16e198c7"
order: 260
---

# Ambientes de execução de plugins e limites de confiança {#plugin-runtimes-and-trust-limits}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Selecione um formato executável cujas dependências e confiança correspondam ao host. Leia a [visão geral dos plugins](index.md), valide o manifesto em relação à compilação real e inspecione restrições específicas do destino antes de carregar código. Compatibilidade de protocolo, compatibilidade de ABI nativa e concessões de permissão são verificações independentes.

## Escolha um ambiente implementado {#choose-an-implemented-runtime}

| Runtime | Execução | Confiança e portabilidade |
| --- | --- | --- |
| `native` | ABI C com `libloading` | Código confiável no mesmo processo; específico do destino |
| `process` | Executável independente, stdin/stdout enquadrados | Isolamento de falhas, permissões comuns do usuário no sistema |
| `napi-vm` | JS compilado antecipadamente no QuickJS via `rquickjs` | Efeitos do host negados por padrão; addons nativos opcionais alteram a confiança |
| `declarative` | Descritores de integração validados | Sem programa convidado; efeitos usam o broker do host |
| `wasm` | Opção de runtime opcional | Mecanismo geral não habilitado nas compilações normais |

A ABI nativa v1 troca bytes serializados por uma pequena tabela C; contêineres Rust, objetos trait e futures não a atravessam. Bibliotecas nativas permanecem carregadas até o encerramento. O enquadramento de processo usa comprimento u32 little-endian seguido de bytes JSON, não RPC de controle delimitado por linhas. Mantenha a saída de diagnóstico fora do canal stdout enquadrado.

## Entenda a compatibilidade QuickJS {#understand-quickjs-compatibility}

O nome de manifesto `napi-vm` permanece estável, mas afirmações antigas sobre um interpretador separado puramente Rust são históricas. Plugins recebem as variáveis globais de compatibilidade documentadas, não um ambiente Node completo. `napi_get_uv_event_loop` não é compatível. Addons nativos `.node` usam um caminho no mesmo processo deliberadamente confiável, permitido por lista e verificado por SHA-256; o nome do runtime JavaScript não os torna seguros.

```sh
bun scripts/task.ts rust test -p tiktools-plugin-loader --lib --locked
bun scripts/task.ts test bun packages
```

Use testes específicos do carregador quando uma configuração completa de addon nativo exigir ferramentas de plataforma indisponíveis.

## Resultado esperado e status {#expected-result-and-status}

A análise do manifesto rejeita combinações incompatíveis de protocolo/ABI antes do carregamento. Android permite QuickJS portátil e entradas válidas de processo Android/Bionic, mas rejeita bibliotecas dinâmicas nativas e addons `.node`. Um artefato Linux/glibc de desktop não é um plugin de processo Android.

## Recuperação {#recovery}

Reinicie um host privado após substituir uma biblioteca nativa. Se a inicialização exceder o tempo limite e ficar incerta, o gerenciador pode manter a identidade do worker e bloquear a substituição; não contorne esse estado declarando sucesso no encerramento. Buffers de eventos são limitados e a entrega pode informar atraso; emissão na inicialização não é uma fila durável. A [implementação do carregador](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-plugin-loader/src) define o mecanismo atual e o comportamento de ciclo de vida.

## Detalhes de operação {#operational-details}

Os limites atuais são memória QuickJS 512 MiB, pilha 2 MiB, orçamento de interrupção por chamada, quadros 16 MiB e até 128 chamadas enfileiradas por instância. Não garantem cancelamento de efeitos. Um processo com timeout é terminado/removido; uma chamada nativa não pode ser morta com segurança e a instância é mantida. Efeitos no SO podem já ter ocorrido. Confira resultado original e ciclo de vida antes de substituir/repetir. Addons `.node` confiáveis usam privilégios do host fora do limite JS.
