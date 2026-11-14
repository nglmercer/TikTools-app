---
title: "Camadas do frontend e fluxo de dados"
docId: "developer.frontend"
translationKey: "developer.frontend"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "40ff5183f34bff831db48c76fef50f9ef8d5fe40b632ed77747c10ebad708fb3"
translationRevision: "e0d2124d9c554a25ae4052f9ff8326bf48e1a5ad4be3949bd9140a1a471c5664"
order: 40
---

# Camadas do frontend e fluxo de dados {#frontend-layers-and-data-flow}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Amplie o aplicativo Vue sem acoplar uma página ao IPC nativo. O frontend usa Vue 3.5, Pinia 4, Vue Router 5 e vue-i18n 11 dos intervalos instalados no workspace. Use as dependências travadas e os comandos de [configuração](setup.md), em vez de um novo esqueleto de frontend.

## Encontre a camada responsável {#find-the-owning-layer}

A estrutura atual de Feature-Sliced Design fica em `src/web/`:

| Camada | Responsabilidade | Exemplo existente |
| --- | --- | --- |
| `app` | Inicialização, roteamento, estrutura e tempo de vida da execução | `app/model/use-app-runtime.ts` |
| `pages` | Composição de rotas | `pages/automations/ui/AutomationsPage.vue` |
| `widgets` | Painéis compostos substanciais | `widgets/live-feed/ui/LiveFeedPanel.vue` |
| `features` | Operações do usuário e seus rascunhos | `features/apply-rule-template/` |
| `entities` | Stores de domínio e lógica de domínio reutilizável | `entities/rule-template/` |
| `shared` | API, primitivas de interface, i18n e utilitários genéricos | `shared/api/control-client.ts` |

Componha camadas superiores a partir das inferiores. Mantenha chamadas ao host por trás do cliente de controle compartilhado; a seleção de transporte pertence à inicialização. Os locais antigos `src/web/platform` e `src/web/views` em artigos legados são históricos.

## Implemente uma alteração focada {#implement-a-focused-change}

1. Localize a rota em `app/router/routes.ts` e siga sua página até o recurso e o store da entidade.
2. Use um componente compartilhado existente para campos, status ou diálogos. Preserve seus rótulos, comportamento de foco e contrato de eventos.
3. Mantenha candidatas editáveis separadas dos registros salvos no host. Use `shared/lib/request-scope.ts` para ignorar respostas assíncronas obsoletas; invalidar não cancela uma mutação aceita pelo host.
4. Adicione chaves de idioma por `shared/i18n` e descarte assinaturas no tempo de vida responsável.

```sh
bun scripts/task.ts test web rule-profiles
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-build
```

## Resultado esperado, status e recuperação {#expected-result-status-and-recovery}

A rota renderiza os dados atuais do host e mantém edições não salvas em conflitos. Rankings tem código e rota, mas permanece oculto na navegação normal; não deduza disponibilidade do produto pela presença de um componente. Os caminhos de inicialização nativo, navegador local e demonstração podem fornecer transportes diferentes usando o mesmo código de recursos.

Se uma resposta obsoleta substituir uma consulta mais recente, confira sua geração antes de alterar o backend. Se uma rota perder sua página de plugin, retorne à visualização de plugins instalados, como a execução já faz. O [runtime do aplicativo](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/app/model/use-app-runtime.ts) é a referência para conexão, ressincronização e desmontagem.

## Detalhes de operação {#operational-details}

Bootstrap escolhe ControlClient real/demo; `mountWorkspace({client,locale,target})` instala Pinia, i18n, router e `controlClientKey`; AppShell inicia assinaturas/resync de gaps e apresentação com `useAppRuntime`. `useControlClient` falha sem provedor mesmo com Pinia/i18n. Reuse a fábrica ou forneça cliente no teste; desmonte e detach ao terminar. Entidades não escolhem transporte.

Antes de await crie ticket e saída da atividade. Aplique resposta só se ticket atual; finalize atividade em finally. Invalide na troca de contexto/desmontagem. Cada operação sobreposta tem sua saída para não apagar busy de outra. Invalidar ticket não cancela trabalho remoto.

```ts
import { createRequestScope, createActivityCounter } from '@/shared/lib/request-scope.ts';
const requests = createRequestScope();
const activity = createActivityCounter();
async function refresh() {
  const ticket = requests.begin();
  const leave = activity.enter();
  try {
    const result = await client.call('system.info', {});
    if (ticket.isCurrent()) info.value = result;
  } finally { leave(); }
}
// onBeforeUnmount(() => { requests.invalidate(); activity.reset(); });
```

```ts
import { mountWorkspace } from '@/app/mount-workspace.ts';
const app = await mountWorkspace({ client, locale: 'en', target: '#app' });
// At teardown:
app.unmount();
client.detach();
```
