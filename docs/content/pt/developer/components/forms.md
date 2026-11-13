---
title: "Formulários"
docId: "developer.components.forms"
translationKey: "developer.components.forms"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "45f58248a1b11283e61965ea97ddca26e664ef977cbe2115a601d3a029abab05"
translationRevision: "52d84516dd29d1f4955184ca16f648bd9dd38a458356b0ee6d6ca5f73ae10877"
---

# Formulários {#forms}

## Objetivo e pré-requisitos {#purpose}

Associe rótulos, dicas e erros ao controle real. As linhas de formulário permitem tanto associação automática quanto atributos explícitos com escopo. Use o workspace Vue existente e seu provedor i18n. A prévia monta os componentes compartilhados reais em um quadro isolado do aplicativo; o tema da documentação não carrega o CSS global do desktop.

## Experimente o exemplo {#example}

1. Carregue a prévia ou abra a demonstração completa.
2. Altere o seletor de estado para inspecionar carregamento, vazio, erro, desativado, texto longo e apresentação móvel quando aplicável. Um estado sem renderização distinta não acrescenta comportamento artificial ao componente.
3. Use Tab, Enter, Escape e as setas conforme o controle. Verifique seu nome acessível, o foco visível e o retorno do foco ao fechar uma janela flutuante.
4. Altere um valor ou execute a ação e inspecione a seleção ou saída resultante. Recarregar ou usar Reset restaura o estado da demonstração.

<DemoFrame scenario="settings" locale="pt" route="/components/forms" title="Exemplo: Formulários" />

## API {#api}

As propriedades, os valores padrão, eventos, slots e declarações de modelo a seguir são gerados a partir do código-fonte do componente. Os tipos importados continuam vinculados a esse código, em vez de serem duplicados manualmente. Os atributos não declarados seguem o contrato de encaminhamento do componente; revise seu código antes de associar um evento nativo a um elemento envolvente.

<!--@include: ../../../../.generated/components/forms.pt.md-->

## Código-fonte executável {#source}

Este mesmo arquivo é compilado para a prévia e incluído como trecho de código. Seu pequeno estado pertence ao exemplo. A lógica de contas, plugins e host permanece em features/entities, fora da interface compartilhada.

<<< ../../../../examples/forms.vue

## Acessibilidade e recuperação {#accessibility}

Use um rótulo visível ou aria-label, preserve a operação pelo teclado e evite depender apenas da cor. Teste em uma largura estreita e com rótulos traduzidos longos. Nos formulários, vincule controlAttrs da linha ao campo real para manter a associação dos erros e do estado obrigatório. Um controle desativado não explica uma falha sozinho; mostre um status adjacente e uma ação de recuperação.

Se uma prévia falhar, feche-a e abra-a novamente, depois use Reset na própria instância da demonstração. Revise a API estática e o código mesmo quando a demonstração estiver indisponível. Diálogos nativos, serviços externos e integração com o sistema operacional exigem testes de desktop separados. [Voltar ao guia de componentes](index.md).

## Detalhes de operação {#operational-details}

`BaseSchemaForm` controla modelo JSON, não persistência. Passe schema, `uiHints.fields`, `fieldOptions`, `fieldErrors`, opcionais `suggestionsFor` e `openMediaPicker`. Hint usa `showIf:{key,equals,notEquals}` para visibilidade; `advanced` agrupa campos opcionais. O proprietário carrega opções dinâmicas e preserva seleção. Erros são por campo; mídia real exige picker fornecido. O exemplo combina textarea/mídia condicional e grupo. Valores aceitos atualizam modelo; salvar ainda exige validação de domínio/host.

`BaseFormRow` infere um input exceto com `group=true`. O slot oferece `controlAttrs:{id,aria-describedby,aria-invalid,aria-required}` ao controle real. Para vários use `group`, nomeie cada filho sem repetir id; ajuda/erro se associam ao grupo. `useSplitAttrs` envia class/style ao contêiner e outros atributos/listeners ao controle. File upload aceita um arquivo e confere tamanho/tipo configurados; falha limpa modelo e mostra erro. Selecionar não é enviar remotamente.
