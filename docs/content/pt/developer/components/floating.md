---
title: "FloatingPanel"
docId: "developer.components.floating"
translationKey: "developer.components.floating"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d061de3feeae9f8cfa6d7e1b5156c7e66795f7e46d64e31f4e2edd933465e52c"
translationRevision: "29da28026befa7cd16324e97c63d66c84f364d7d4d680fcec813b75d7c79bb1e"
---

# FloatingPanel {#floatingpanel}

## Objetivo e pré-requisitos {#purpose}

Mantenha os diagnósticos disponíveis enquanto a área de trabalho muda. Mover, minimizar e restaurar o painel deve funcionar com teclado e ponteiro. Use o workspace Vue existente e seu provedor i18n. A prévia monta os componentes compartilhados reais em um quadro isolado do aplicativo; o tema da documentação não carrega o CSS global do desktop.

## Experimente o exemplo {#example}

1. Carregue a prévia ou abra a demonstração completa.
2. Altere o seletor de estado para inspecionar carregamento, vazio, erro, desativado, texto longo e apresentação móvel quando aplicável. Um estado sem renderização distinta não acrescenta comportamento artificial ao componente.
3. Use Tab, Enter, Escape e as setas conforme o controle. Verifique seu nome acessível, o foco visível e o retorno do foco ao fechar uma janela flutuante.
4. Altere um valor ou execute a ação e inspecione a seleção ou saída resultante. Recarregar ou usar Reset restaura o estado da demonstração.

<DemoFrame scenario="settings" locale="pt" route="/components/floating" title="Exemplo: FloatingPanel" />

## API {#api}

As propriedades, os valores padrão, eventos, slots e declarações de modelo a seguir são gerados a partir do código-fonte do componente. Os tipos importados continuam vinculados a esse código, em vez de serem duplicados manualmente. Os atributos não declarados seguem o contrato de encaminhamento do componente; revise seu código antes de associar um evento nativo a um elemento envolvente.

<!--@include: ../../../../.generated/components/floating.pt.md-->

## Código-fonte executável {#source}

Este mesmo arquivo é compilado para a prévia e incluído como trecho de código. Seu pequeno estado pertence ao exemplo. A lógica de contas, plugins e host permanece em features/entities, fora da interface compartilhada.

<<< ../../../../examples/floating.vue

## Acessibilidade e recuperação {#accessibility}

Use um rótulo visível ou aria-label, preserve a operação pelo teclado e evite depender apenas da cor. Teste em uma largura estreita e com rótulos traduzidos longos. Nos formulários, vincule controlAttrs da linha ao campo real para manter a associação dos erros e do estado obrigatório. Um controle desativado não explica uma falha sozinho; mostre um status adjacente e uma ação de recuperação.

Se uma prévia falhar, feche-a e abra-a novamente, depois use Reset na própria instância da demonstração. Revise a API estática e o código mesmo quando a demonstração estiver indisponível. Diálogos nativos, serviços externos e integração com o sistema operacional exigem testes de desktop separados. [Voltar ao guia de componentes](index.md).
