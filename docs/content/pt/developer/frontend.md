---
title: "Camadas do frontend e fluxo de dados"
docId: "developer.frontend"
translationKey: "developer.frontend"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "289f2dc81c1eebfe9d93378ec7ca3dcf67413524a86f032651762e1f0b116a61"
translationRevision: "289f2dc81c1eebfe9d93378ec7ca3dcf67413524a86f032651762e1f0b116a61"
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
