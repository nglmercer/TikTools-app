---
title: "Internacionalização e acessibilidade"
docId: "developer.i18n-accessibility"
translationKey: "developer.i18n-accessibility"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7dc918a0278876f1490f546683b9cf084f00058f7650b72521f98bc87fbfe517"
translationRevision: "7dc918a0278876f1490f546683b9cf084f00058f7650b72521f98bc87fbfe517"
order: 120
---

# Internacionalização e acessibilidade {#internationalization-and-accessibility}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Adicione uma tela que continue compreensível nos idiomas disponíveis e utilizável por teclado, toque e tecnologia assistiva. Leia [camadas do frontend](frontend.md) e use a interface compartilhada existente. Os idiomas do aplicativo em execução e as edições publicadas da documentação são sistemas separados.

## Adicione conteúdo localizado {#add-localized-content}

O catálogo do aplicativo fica em `src/web/shared/i18n/`. O inglês é a alternativa tipada; os outros catálogos disponíveis carregam sob demanda. `activateLocale` carrega as mensagens escolhidas e atualiza `document.documentElement.lang`. Armazene uma preferência de idioma pelo store de idioma existente em vez de gravar outra chave no armazenamento local.

1. Adicione a chave semântica da mensagem ao catálogo em inglês e aos catálogos/módulos equivalentes disponíveis.
2. Mantenha IDs de API, caminhos de eventos e identificadores de recursos armazenados estáveis; traduza seus rótulos visíveis.
3. Use interpolação para texto variável e auxiliares compartilhados de internacionalização para valores de exibição.
4. Para descritores de plugins, use o contrato existente de texto localizado `{default,i18key}` e os catálogos mesclados pelo host. Nunca renderize strings traduzidas ou fornecidas por plugins como HTML.

## Preserve a semântica dos controles {#preserve-control-semantics}

Prefira `BaseTextField`, `BaseFormRow`, `BaseDialog`, abas e componentes de status. Um texto de exemplo no campo não substitui um rótulo acessível. Botões com ícones precisam de um nome localizado. A validação deve identificar o campo e expor o estado inválido; não comunique falhas apenas pela cor. Preserve retorno de foco do diálogo, comportamento de Escape, seleção pelo teclado e contornos visíveis de foco.

```sh
bun scripts/task.ts test web i18n
bun scripts/task.ts test web catalogs
bun scripts/task.ts test web BaseDialog
bun scripts/task.ts frontend-lint
```

Para revisão manual, percorra todo o fluxo com Tab, abra/feche o diálogo, troque de idioma enquanto ele estiver aberto e repita em largura estreita. Ações de status reveladas ao passar o ponteiro também devem aparecer com foco de teclado e continuar disponíveis por toque.

## Resultado esperado e status {#expected-result-and-status}

Tipagem de catálogos e verificações de paridade detectam chaves ausentes; controles compartilhados fornecem comportamento implementado de teclado e ARIA. Eles não constituem uma certificação geral de acessibilidade de toda tela composta. Rankings permanece oculto mesmo quando existem strings traduzidas.

## Recuperação {#recovery}

Se aparecer texto não traduzido, confira o carregamento sob demanda dos catálogos e a grafia exata da chave antes de adicionar alternativas em inglês nos componentes. Se o foco desaparecer, inspecione o controle que recebe atributos encaminhados e o alvo de retorno do diálogo. A [implementação i18n](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/i18n/i18n.ts) e os testes de componentes definem o comportamento atual; [componentes](components/index.md) explica os pontos de entrada compartilhados.
