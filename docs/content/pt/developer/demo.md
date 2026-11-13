---
title: "Arquitetura e limites da demonstração no navegador"
docId: "developer.demo"
translationKey: "developer.demo"
locale: "pt"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "faaecb5fb98c82d290108d1d9ffc8375e8f0fcd04600da60557a63272ad57d59"
translationRevision: "faaecb5fb98c82d290108d1d9ffc8375e8f0fcd04600da60557a63272ad57d59"
order: 140
---

# Arquitetura e limites da demonstração no navegador {#browser-demo-architecture-and-limits}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Explore a interface real do aplicativo com dados sintéticos em um navegador. Use Bun 1.4.4 e a instalação travada do repositório. A entrada explícita da demonstração funciona independentemente de Rust, contas e de um host em execução. Seu runtime e suas verificações de aceitação no navegador fazem parte deste repositório.

## Compile e inspecione {#build-and-inspect}

```sh
bun scripts/task.ts docs check
bun scripts/task.ts demo dev
bun scripts/task.ts demo build
bun scripts/task.ts site build
bun scripts/task.ts site preview
```

Para uma prévia local sem commit, use `TIKTOOLS_SITE_ALLOW_DIRTY=1 bun scripts/task.ts site build`. Seu manifesto fica marcado como sujo e a publicação o rejeita. A montagem final exige que os dois bundles informem o mesmo SHA Git limpo e a mesma base; recompile após o commit. Execute um servidor de desenvolvimento ou prévia somente quando o trabalho local de navegador for desejado. `docs` é responsável pelo VitePress e `demo` pelo bundle do aplicativo de navegador; `site build` os monta em um site estático. O workspace de documentação mantém sua dependência VitePress separada das ferramentas Vite 8 do aplicativo. Comandos não devem instalar dependências silenciosamente nem invocar Cargo.

## Preserve o limite do aplicativo {#preserve-the-application-boundary}

`src/web/app/mount-workspace.ts` fornece a fábrica compartilhada de app/Pinia/roteador; importá-la não monta nem escolhe um transporte. A inicialização explícita da demonstração fornece `src/web/demo/runtime/transport.ts` no limite compartilhado de `ControlTransport`/cliente de controle. Dados iniciais puros ficam em `demo/fixtures/base-host.ts`, independentemente do Playwright. `shared/lib/app-storage.ts` e `demo/storage.ts` fornecem adaptadores de armazenamento isolados para que redefinições não limpem preferências nativas nem outra instância da demonstração. Páginas, widgets e recursos devem usar seus stores de entidades e chamadas de `src/web/shared/api` existentes. Não crie um segundo aplicativo de imitação, injete APIs nativas falsas na inicialização nativa comum nem deixe requisições da demonstração chegarem a um host real.

`demo/scenarios.ts` valida 11 IDs de cenário e as rotas/idiomas/tutoriais permitidos. `runtime/host.ts` registra manipuladores explícitos de métodos; a descoberta lista esses manipuladores, IDs de requisição são repetidos em `rpc-response` e efeitos incompatíveis retornam erro imediatamente. O host inicializa registros fictícios e avança um relógio fixo um passo por Next event. Play e Next event primeiro conectam o criador fictício pelo RPC simulado quando necessário e depois entregam um evento imediato. Play continua a cada dois segundos; Pause para a produção. A barra de ferramentas exibe o estado da reprodução e a contagem de eventos. O fluxo para em 300 passos e mantém no máximo 60 eventos de feed e 60 execuções. Filas de resposta e registros armazenados têm limites separados.

Reset limpa apenas o prefixo atual `tiktools.demo.<instance>.*` e recarrega o quadro. Recarregar redefine singletons de módulos, diários, i18n e diagnósticos. Quadros incorporados diferentes recebem IDs distintos de instância. Sair da página desmonta Vue, descarta stores, desconecta cliente/transporte e limpa os dois temporizadores. A demonstração não instala uma ponte `window.ipc` nem realiza HTTP arbitrário, shell, RCON, execução de scripts de visitantes ou instalação nativa de atualizações. Sua CSP rejeita conexões de rede. Modelos de registro e simulação de filtros usam funções compartilhadas puras; são exemplos do contrato de frontend, não substitutos da execução Rust.

A entrada compartilhada `startTour(id, {router, capabilities, taskState})` e o `tourRegistry` exportado ficam em `src/web/features/guided-help/index.ts`. Sete tutoriais versionados usam alvos estáveis de página e catálogos separados EN/ES/PT/FR. Eles carregam Driver.js apenas sob solicitação, restringem alvos ao workspace, respeitam movimento reduzido, restauram o foco e cancelam esperas limitadas na navegação. Dispensar nunca conclui uma tarefa nem executa uma ação do aplicativo. A seleção de tutorial nomeia a tarefa completa, prepara o cenário correspondente e pausa a reprodução antes de iniciar. O controle de fechar do Driver dispensa o guia; a barra de ferramentas informa o resultado. Reset limpa o progresso versionado desta instância.

## Resultado esperado e status {#expected-result-and-status}

A demonstração apresenta navegação, edição local, apresentação de eventos e estados de recuperação compatíveis com seu modelo de cenários. Ela não valida sessões reais do TikTok, fonte de navegador do OBS, comportamento Wry/nativo, backend de produção nem vários PCs. Rankings permanece oculto; Changes de Automations é apenas local, Variables revisadas são trabalho nativo real do produto, Changes de Overlays continua como uma continuação não mesclada e os controles de instalação automática do atualizador são experimentais.

## Componentes e Markdown {#components-and-markdown}

`docs/examples/*.vue` fornece tanto o catálogo renderizado quanto os trechos de código-fonte. Botões pequenos renderizam diretamente no Markdown sem estilos globais de desktop. Diálogos com estado, exemplos de layout e páginas do aplicativo usam `DemoFrame`, que constrói URLs validadas e compatíveis com a base e carrega um iframe apenas sob solicitação. A opção pública `VITE_TIKTOOLS_PUBLIC_UI_KIT=1` expõe o catálogo de componentes sem habilitar todas as ferramentas de desenvolvimento do desktop.

## Recuperação e verificação {#recovery-and-verification}

Redefina um cenário após uma lacuna ou falha simulada intencionalmente. Se ele contatar um serviço externo, pare e corrija o limite do transporte. Verifique conteúdo, compilação da demonstração e montagem do site independentemente; uma compilação de documentação não comprova correção nativa. [Testes](testing.md) distingue evidências de host simulado, navegador, Rust e aceitação manual.
