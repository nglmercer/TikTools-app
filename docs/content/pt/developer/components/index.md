---
title: "Componentes compartilhados de interface e composição"
docId: "developer.components.index"
translationKey: "developer.components.index"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "4c70fe5256ba711f6f8fd6c3e8d305c4a44bdd6ba06092a846015e6f1898744f"
translationRevision: "4c70fe5256ba711f6f8fd6c3e8d305c4a44bdd6ba06092a846015e6f1898744f"
order: 290
---

# Componentes compartilhados de interface e composição {#shared-ui-components-and-composition}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Componha um recurso com o kit de interface existente, preservando a interação e a localização. Use as ferramentas instaladas no workspace Vue e leia o [mapa do frontend](../frontend.md). Esta página apresenta o kit; o código-fonte, as propriedades, os testes e a referência específica da família de cada componente continuam sendo a autoridade para sua API exata.

## Encontre a primitiva correta {#find-the-correct-primitive}

`src/web/shared/ui/` contém botões, campos de texto/data/cor, áreas de texto, caixas de seleção, interruptores, caixas de combinação, abas, tabelas, formulários, diálogos, status, alertas e controles estruturais. Os tokens de design ficam em `src/web/app/styles/tokens.css`; os estilos base e responsivos compõem o aplicativo. Use tokens para espaçamento, cores, tipografia e tratamento do foco, em vez de criar um segundo tema em cada recurso.

1. Encontre um recurso existente com a interação de que você precisa.
2. Leia `defineProps`, slots, valores emitidos/de modelo e o teste da primitiva antes de envolvê-la.
3. Mantenha validação de domínio, estado salvo e chamadas ao host no recurso ou entidade responsável.
4. Encaminhe rótulos acessíveis e atributos do controle pelo contrato de encaminhamento aceito pela primitiva.

Um componente com script-setup pode usar a API real do botão:

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const { t } = useI18n();
</script>

<template>
  <BaseButton variant="primary" type="submit">{{ t('common.apply') }}</BaseButton>
</template>
```

O formulário que o contém é responsável pelo envio e pela validação. `BaseButton` aceita as variantes `default`, `primary`, `danger` e `ghost`; uma cor de perigo não constitui, por si só, um fluxo de confirmação.

## Resultado esperado e status {#expected-result-and-status}

As primitivas compartilhadas oferecem comportamento implementado e consistente de teclado, foco e ARIA. A composição ainda precisa de revisão: textos de exemplo nos campos não são rótulos, controles apenas com ícones precisam de nomes, estados desativados devem refletir quem controla a operação atual e erros assíncronos devem permanecer visíveis. A rota de desenvolvimento do kit de interface é uma demonstração, não um recurso separado do produto nem um teste de aceitação do backend.

## Verifique e recupere {#verify-and-recover}

Execute `bun scripts/task.ts test web BaseButton`, o teste específico do componente alterado e `bun scripts/task.ts frontend-lint`. Verifique largura estreita, navegação pelo teclado e rótulos traduzidos. Se um atributo chegar ao elemento envolvente em vez do campo, inspecione `shared/lib/use-split-attrs.ts` e o teste do controle antes de contornar a primitiva. Consulte [acessibilidade](../i18n-accessibility.md) e o [código da interface](https://github.com/nglmercer/TikTools-app/tree/remake/src/web/shared/ui) ao adaptar um exemplo de referência de família.

## Referências interativas das famílias {#families}

- [Botões e ações](buttons.md)
- [Menus](menus.md)
- [TextField e SpinBox](fields.md)
- [ComboBox e SearchSelect](selects.md)
- [Interruptores](switches.md)
- [Formulários](forms.md)
- [Abas e barras de ferramentas](tabs.md)
- [Listas e tabelas](collections.md)
- [Filtros e seleção](filters.md)
- [Alertas, estados vazios e status](feedback.md)
- [Painéis e diálogos](panels.md)
- [FloatingPanel](floating.md)
- [Splitter e MasterDetail](layouts.md)
- [Visualização e comparação de JSON](json.md)
- [Importação e revisão](import-review.md)
- [Padrões de revisão do Cloud](cloud-patterns.md)
- [Padrões de overlays](overlay-patterns.md)
