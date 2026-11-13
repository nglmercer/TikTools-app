---
title: "Ciclo de vida, propriedade e limpeza"
docId: "developer.lifecycle"
translationKey: "developer.lifecycle"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "92e2164c28be287f8b5d5c96d787a106bb011e1939ff490e4e69737c87bb9716"
translationRevision: "92e2164c28be287f8b5d5c96d787a106bb011e1939ff490e4e69737c87bb9716"
order: 50
---

# Ciclo de vida, propriedade e limpeza {#lifecycle-ownership-and-cleanup}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Faça montagem, reconexão e encerramento liberarem os recursos pelos quais são responsáveis. Conheça a [arquitetura](architecture.md) e use um `TIKTOOLS_HOME` isolado para experimentos nativos de ciclo de vida. Cancelar uma assinatura do frontend e encerrar o host resolvem problemas de propriedade diferentes.

## Siga o tempo de vida do desktop {#follow-the-desktop-lifetime}

`crates/tiktools-desktop/src/app.rs` implementa o manipulador de aplicativo do Winit. A criação de janela e WebView ocorre na thread do laço de eventos. A janela começa oculta e aparece depois que Vue envia `frontend-ready`. Fechar oculta a janela quando há bandeja; sem bandeja, inicia o encerramento. Sair pede ao núcleo para parar seus serviços antes de encerrar o laço de eventos.

`crates/tiktools-core/src/shutdown.rs` fecha cedo a admissão de inicialização de plugins, para coordenação em segundo plano e monitoramento, desconecta a LIVE e drena o trabalho sob sua responsabilidade. Chamadores repetidos de encerramento aguardam a mesma conclusão. Descartar um future não pode parar código nativo arbitrário nem desfazer um efeito externo.

## Gerencie recursos do frontend explicitamente {#own-frontend-resources-explicitly}

Use o padrão já seguido por `use-app-runtime.ts`:

```ts
import { onUnmounted } from 'vue';
import { useControlClient } from '@/shared/api/use-control-client.ts';

const control = useControlClient();
const stop = control.onTopic('points.changed', () => {
  // Atualize aqui a apresentação confirmada do store responsável.
});
onUnmounted(stop);
```

O corpo do callback pertence à entidade responsável pela visualização. Um recurso deve descartar suas assinaturas, temporizadores, quadros de animação e escopos de requisição, enquanto a inicialização do aplicativo é responsável por conectar e desconectar o transporte compartilhado. Alterações de modelo de widget devem descartar o controlador anterior em vez de deixar seus temporizadores de alerta executando.

## Resultado esperado e verificação {#expected-result-and-verification}

Sair de uma página e voltar não multiplica callbacks. Quando oculta, a apresentação interrompe renderizações desnecessárias sem pausar a contabilização autoritativa. Reconectar informa lacunas e recarrega instantâneos; não reproduz efeitos de negócio. Execute `bun scripts/task.ts test web cloud-lifecycle` e `bun scripts/task.ts test web request-scope` para exemplos específicos.

## Recuperação e limites {#recovery-and-limits}

Uma inicialização QuickJS que excedeu o tempo limite pode permanecer incerta; o gerenciador mantém a identidade do worker e bloqueia a substituição em vez de fingir que a parada funcionou. Alguns caminhos síncronos de controle do ciclo de vida de plugins ainda podem bloquear o chamador. Trate-os como limites de implementação, especialmente durante encaminhamento ao atualizador. Pare corretamente o host proprietário em vez de excluir seu bloqueio ativo. O [código de encerramento](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/shutdown.rs) e os testes de execução são evidências mais fortes que um diagrama histórico de ciclo de vida.
