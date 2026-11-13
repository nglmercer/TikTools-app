---
title: "Desempenho e trabalho limitado"
docId: "developer.performance"
translationKey: "developer.performance"
locale: "pt"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "04bc0a89b86724872adae3ab8ddc8e29f5517dad14d51bc103d917582cf289ae"
translationRevision: "05ab61ca49c64e95b15cb947f32042adae5820de15ca8999b2fb49b6f7aa0e93"
order: 110
---

# Desempenho e trabalho limitado {#performance-and-bounded-work}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Meça um gargalo concreto preservando contabilização de eventos e encerramento. Comece com compilação fixa, entrada determinística e perfil isolado. Leia [eventos](events.md) para não confundir um feed mais rápido com persistência mais confiável. Números históricos de benchmark são observações da compilação e máquina identificadas, não garantias atuais do produto.

## Meça o limite correto {#measure-the-right-boundary}

A outbox do desktop separa apresentação com perdas de respostas confiáveis. Instantâneos podem ser combinados; estouro da via confiável deve falhar visivelmente. Entrada de LIVE, trabalho de plugins, chamadas de processadores e admissão de automações têm seus próprios limites. Aumentar uma fila pode transferir pressão de memória ou latência para a próxima etapa.

O runtime do frontend expõe diagnósticos agregados em desenvolvimento ou quando compilado com `VITE_TIKTOOLS_DIAGNOSTICS=1`. Seu instantâneo inclui linhas renderizadas, percentis de intervalo entre quadros, chamadas pendentes do cliente de controle, imagens e temporizadores sob sua responsabilidade. Dados de heap JavaScript podem estar indisponíveis em determinada WebView. Páginas ocultas e visibilidade do host governam o interesse de apresentação, não a contabilização de negócio.

## Execute uma comparação focada {#run-a-focused-comparison}

1. Registre revisão da compilação, plataforma, composição de eventos, imagens ativadas, aba visível e duração.
2. Estabeleça uma referência com os mesmos dados de teste e período de amostragem.
3. Altere um limite e compare vazão, latência p95/p99, descartes de fila, lacunas, memória retida e drenagem no encerramento.
4. Inclua um plugin ou consumidor lento e uma fila saturada; um benchmark ocioso não testa contrapressão.

```sh
bun scripts/task.ts test web live-feed-state
bun scripts/task.ts rust test -p tiktools-core --features persistence,native-tiktok --test live_throughput --locked
```

Inspecione requisitos de recursos antes de ativar um benchmark nativo; verificações unitárias específicas comuns não precisam de conexão real com TikTok. Use `crates/tiktools-core/tests/live_throughput.rs` e os testes da outbox do desktop como referências de implementação.

## Resultado esperado e status {#expected-result-and-status}

Caminhos limitados de projeção e admissão estão implementados, com contadores e relato de lacunas. O [gravador ordenado atual](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/points/queue.rs) rejeita gravações saturadas antes de alterar a memória e proíbe a alternativa de gravação síncrona. Relatórios antigos de desempenho descrevem defeitos de ordenação já superados. Avalie limites de cache e recuperação de falhas pela implementação/testes atuais em vez de deduzir correção completa de uma memória de interface limitada.

## Recupere {#recover}

Se os quadros melhorarem enquanto a integridade da contabilização piorar, reverta a alteração no limite e inspecione os resultados do gravador/admissão. Se temporizadores de abas ocultas crescerem, siga a [limpeza de ciclo de vida](lifecycle.md). Mantenha contagem de entrada bruta, contagem aceita e classificação de perdas no relatório. Uma lacuna pode recuperar estado de exibição; não pode reparar um efeito rejeitado ou que ficou incerto.
