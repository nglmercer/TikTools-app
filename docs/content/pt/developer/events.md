---
title: "Normalização de eventos, lotes e lacunas"
docId: "developer.events"
translationKey: "developer.events"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "caded5529a62e826a91d2deb86936c29c709ac713c2a0cd021f467b1dfa2b6e7"
translationRevision: "701c89daaff0b607f1dece32da0ca5d9f3162055ae402fadafc709d39f970188"
order: 100
---

# Normalização de eventos, lotes e lacunas {#event-normalization-batches-and-gaps}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Consuma eventos de LIVE sem contagem duplicada nem confundir recuperação de apresentação com execução. Entenda a [persistência](storage.md) e use dados de teste sanitizados em vez de capturas de contas reais. `tiktools-tiktok` envolve as dependências fixadas de assinatura/decodificação e exporta valores estáveis do TikTools; o núcleo não expõe objetos protobuf como API do aplicativo.

## Siga o pipeline implementado {#follow-the-implemented-pipeline}

A entrada captura o contexto original de conexão de criador/sala antes do trabalho assíncrono. A normalização produz valores de chat, presentes, curtidas, atividade social e ciclo de vida. O núcleo publica originais e gerencia o trabalho de contabilização/efeitos separadamente. Processadores podem associar evidências derivadas limitadas em `event.intel`; falhas preservam eventos brutos com diagnósticos de degradação. Contratos gerados de automação descrevem apenas campos emitidos.

A identidade de repetição do backend inclui criador, sala, método, tipo e um ID de mensagem diferente de zero. IDs ausentes não permitem identificar repetições. A contabilização de combos de presentes usa identidade de remetente/presente/grupo e máximos cumulativos, para que um pacote final não atribua a mesma quantidade duas vezes. Preços desconhecidos permanecem zero. Transições de agregados podem apoiar o cruzamento de um limite com `previous < threshold <= total`; não substituem os originais públicos.

## Implemente um consumidor {#implement-a-consumer}

1. Use tipos de `src/automation/contracts/generated/` ou o analisador de envelope de domínio de widgets para esse limite.
2. Preserve cursores de entrega durante a serialização e elimine sobreposição ao combinar tempo real e histórico.
3. Processe lotes ordenados sem emitir uma renderização reativa por evento.
4. Exponha sinais explícitos de lacuna/sobrecarga. Recupere a apresentação com páginas limitadas ou um instantâneo novo, nunca invocando pontos ou ações novamente.

```sh
bun scripts/task.ts contracts --check
bun scripts/task.ts test web live-feed-state
bun scripts/task.ts test bun automation
```

A [fonte dos contratos Rust](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-core/src/contracts) gera o registro de eventos, esquemas e amostras; não reproduza tabelas completas de campos de eventos manualmente.

## Resultado esperado e status {#expected-result-and-status}

A sobreposição de reconexão só é deduplicada enquanto o evento identificado permanecer no armazenamento finito atual. Filas, contadores, histórico e caches de repetição permanecem limitados. Isso não é um arquivo ilimitado de eventos nem uma garantia universal de execução exatamente uma vez. Esgotamento de capacidade e persistência indisponível têm consequências explícitas para a integridade.

## Recupere e verifique {#recover-and-verify}

Use dados sanitizados de presentes/emotes e teste pacotes finais de combo, IDs ausentes, identidade entre salas e lacunas de reconexão. Inspecione a revisão atual do assinador em `Cargo.toml` em vez de copiar um hash de um relatório antigo de confiabilidade. Métodos desconhecidos do serviço continuam desconhecidos até que a normalização e os contratos gerados os adicionem deliberadamente.

## Detalhes de operação {#operational-details}

Os padrões atuais (2026-11-13) são limites de implementação, não SLA. A entrada LIVE comporta 2.048 itens e a fila de efeitos 128, não orçamentos de bytes. Eventos de ciclo de vida aguardam capacidade. Presentes/follows/shares aguardam até um segundo antes do descarte com lacuna; chat/likes/joins/outros dados são descartados imediatamente na saturação. Confira `droppedCritical`, lotes/unidades de likes e perdas de efeitos: uma entrada descartada pode nunca ter executado efeitos. Backfill recupera apenas apresentação, nunca pontos ou ações.

A deduplicação guarda até 4.096 identidades por 300 segundos em memória do processo, entre reconexões enquanto o armazenamento existir. Expiração, expulsão por capacidade, IDs ausentes ou reinício removem a proteção. A proteção contra sobreposição vale somente nesses limites.

`ui.feed.backfill` recebe `{version:1,after:0,limit:64}`, cursor numérico global e limite 1–512. Confira `oldestCursor`, `latestCursor`, `gap` e `truncated`; continue do último cursor recebido. `ui.feed.creator` usa `{creatorId,streamId,after,limit}`, cursores textuais por stream/criador, limite 1–64 e orçamento de bytes. Não misture os cursores. Se o stream mudar/for expulso, o cursor estiver no futuro, houver lacuna ou `CreatorFeedUnavailable`, atualize a apresentação autoritativa e sinalize histórico incompleto, sem reconstruir efeitos.

TikTok inicializa sessão, sala, catálogo de presentes, bundle de assinatura e socket. Configuração: `TIKTOOLS_TIKTOK_SIGNING_BUNDLE`, `TIKTOOLS_TIKTOK_BUNDLE_CACHE`, `TIKTOOLS_TIKTOK_BUNDLE_URL`. Padrões: cinco tentativas, espera inicial 2 s até 30 s. HTTP 429 usa Retry-After ou mínimo crescente 30–300 s com jitter, até duas novas tentativas por rate limit, mantendo cooldown após cancelar/esgotar. Outros casos exigem falha transitória de transporte. Inspecione `Reconnecting { attempt, delay_ms }` e a etapa; não inclua cookies ou material privado de assinatura.

```sh
tiktools api call system.health '{}' --json
```
