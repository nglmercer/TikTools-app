---
title: "ComboBox e SearchSelect"
docId: "developer.components.selects"
translationKey: "developer.components.selects"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6dc5181ece21f73c6569b5d8e54c896c6df86390333a5240ef0696257e5e56d5"
translationRevision: "6e19d340be14c921f5c39ad15a02bd1327f177e7b36b2c5c42db4bf07cc85324"
---

# ComboBox e SearchSelect {#combobox-and-searchselect}

## Objetivo e pré-requisitos {#purpose}

Escolha um valor estável entre opções rotuladas. SearchSelect busca rótulos e valores; um valor selecionado desconhecido é mostrado diretamente. Use o workspace Vue existente e seu provedor i18n. A prévia monta os componentes compartilhados reais em um quadro isolado do aplicativo; o tema da documentação não carrega o CSS global do desktop.

## Experimente o exemplo {#example}

1. Carregue a prévia ou abra a demonstração completa.
2. Altere o seletor de estado para inspecionar carregamento, vazio, erro, desativado, texto longo e apresentação móvel quando aplicável. Um estado sem renderização distinta não acrescenta comportamento artificial ao componente.
3. Use Tab, Enter, Escape e as setas conforme o controle. Verifique seu nome acessível, o foco visível e o retorno do foco ao fechar uma janela flutuante.
4. Altere um valor ou execute a ação e inspecione a seleção ou saída resultante. Recarregar ou usar Reset restaura o estado da demonstração.

<DemoFrame scenario="settings" locale="pt" route="/components/selects" title="Exemplo: ComboBox e SearchSelect" />

## API {#api}

As propriedades, os valores padrão, eventos, slots e declarações de modelo a seguir são gerados a partir do código-fonte do componente. Os tipos importados continuam vinculados a esse código, em vez de serem duplicados manualmente. Os atributos não declarados seguem o contrato de encaminhamento do componente; revise seu código antes de associar um evento nativo a um elemento envolvente.

<!--@include: ../../../../.generated/components/selects.pt.md-->

## Código-fonte executável {#source}

Este mesmo arquivo é compilado para a prévia e incluído como trecho de código. Seu pequeno estado pertence ao exemplo. A lógica de contas, plugins e host permanece em features/entities, fora da interface compartilhada.

<<< ../../../../examples/selects.vue

## Acessibilidade e recuperação {#accessibility}

Use um rótulo visível ou aria-label, preserve a operação pelo teclado e evite depender apenas da cor. Teste em uma largura estreita e com rótulos traduzidos longos. Nos formulários, vincule controlAttrs da linha ao campo real para manter a associação dos erros e do estado obrigatório. Um controle desativado não explica uma falha sozinho; mostre um status adjacente e uma ação de recuperação.

Se uma prévia falhar, feche-a e abra-a novamente, depois use Reset na própria instância da demonstração. Revise a API estática e o código mesmo quando a demonstração estiver indisponível. Diálogos nativos, serviços externos e integração com o sistema operacional exigem testes de desktop separados. [Voltar ao guia de componentes](index.md).

## Detalhes de operação {#operational-details}

Forneça a opção selecionada durante carga para exibir seu rótulo; se faltar, aparece o valor bruto. `search` emite a consulta para busca remota opcional do proprietário, mantendo busca interna por rótulo/valor. Setas/Home/End percorrem opções habilitadas; Enter escolhe a linha. Com `allowCustom`, confirmar texto remove espaços externos; Escape cancela. Teste ID não carregado, linhas desativadas e valor personalizado além da seleção normal.

## Detalhes de operação {#keyboard-contract}

Revise [teclado, exceções do diálogo e nomes/tooltip](../i18n-accessibility.md#operational-details). Confira foco inicial/retorno no fluxo completo, incluindo diálogos não fecháveis e editores desmontados.
