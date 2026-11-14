---
title: "Arquitetura e limites da demonstração no navegador"
docId: "developer.demo"
translationKey: "developer.demo"
locale: "pt"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "9884cf56b87f6ddb7a6a9331e144da20913db78eb4d6f764fbd00c7a17f00f31"
translationRevision: "7b39723bfcf09e9de5a8e38a0c5abbae999af5925818338170ffcbcf8e5cd055"
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

O parâmetro de consulta `embed=connection|feed|automation|points|overlays` seleciona um exemplo focado para incorporação. Ele omite a navegação geral, mantendo os componentes, as páginas e o runtime reais. Use a rota correspondente ao painel escolhido e não combine esse modo com um tutorial. Todos os iframes da documentação também recebem `framed=1`, que oculta a barra grande `DemoToolbar` dentro do quadro. Os controles compactos de reproduzir, pausar, próximo evento e reiniciar ficam no componente pai `DemoFrame`. A demonstração independente mantém sua barra completa; o link para abri-la remove os parâmetros de incorporação.

A comunicação entre `DemoFrame` e o iframe verifica a origem, a janela de origem e o identificador da instância. Essa ponte aceita apenas os comandos limitados de reprodução, reinício e seleção de casos predefinidos de recuperação da conexão; ela não expõe chamadas RPC arbitrárias. O transporte de controle continua isolado no runtime da demonstração.

Reset limpa apenas o prefixo atual `tiktools.demo.<instance>.*` e recarrega o quadro. Recarregar redefine singletons de módulos, diários, i18n e diagnósticos. Quadros incorporados diferentes recebem IDs distintos de instância. Sair da página desmonta Vue, descarta stores, desconecta cliente/transporte e limpa os dois temporizadores. A demonstração não instala uma ponte `window.ipc` nem realiza HTTP arbitrário, shell, RCON, execução de scripts de visitantes ou instalação nativa de atualizações. Sua CSP rejeita conexões de rede. Modelos de registro e simulação de filtros usam funções compartilhadas puras; são exemplos do contrato de frontend, não substitutos da execução Rust.

A entrada compartilhada `startTour(id, {router, capabilities, taskState})` e o `tourRegistry` exportado ficam em `src/web/features/guided-help/index.ts`. Sete tutoriais versionados usam alvos estáveis de página e catálogos separados para cada idioma compatível. Eles carregam Driver.js apenas sob solicitação, restringem alvos ao workspace, respeitam movimento reduzido, restauram o foco e cancelam esperas limitadas na navegação. Dispensar nunca conclui uma tarefa nem executa uma ação do aplicativo. A seleção de tutorial nomeia a tarefa completa, prepara o cenário correspondente e pausa a reprodução antes de iniciar. O controle de fechar do Driver dispensa o guia; a barra de ferramentas informa o resultado. Reset limpa o progresso versionado desta instância.

## Resultado esperado e status {#expected-result-and-status}

A demonstração apresenta navegação, edição local, apresentação de eventos e estados de recuperação compatíveis com seu modelo de cenários. Ela não valida sessões reais do TikTok, fonte de navegador do OBS, comportamento Wry/nativo, backend de produção nem vários PCs. Rankings permanece oculto; Changes de Automations é apenas local, Variables revisadas são trabalho nativo real do produto, Changes de Overlays continua como uma continuação não mesclada e os controles de instalação automática do atualizador são experimentais.

## Componentes e Markdown {#components-and-markdown}

`docs/examples/*.vue` fornece tanto o catálogo renderizado quanto os trechos de código-fonte. Botões pequenos renderizam diretamente no Markdown sem estilos globais de desktop. Diálogos com estado, exemplos de layout e páginas do aplicativo usam `DemoFrame`, que constrói URLs validadas e compatíveis com a base. Nas páginas de artigos, o comportamento padrão carrega o iframe apenas sob solicitação. A opção pública `VITE_TIKTOOLS_PUBLIC_UI_KIT=1` expõe o catálogo de componentes sem habilitar todas as ferramentas de desenvolvimento do desktop.

Para colocar um painel de conexão junto às instruções de uma página em português, use:

```vue
<DemoFrame scenario="connection" locale="pt" panel="connection" title="Pratique sua primeira conexão" />
```

A propriedade `panel` aceita `connection`, `feed`, `automation`, `points` ou `overlays`. Por padrão, um convite neutro para iniciar o exemplo aparece antes da interação; ele não imita a interface do aplicativo. O iframe só é carregado quando o leitor escolhe experimentar.

Na página inicial, `DemoFrame` usa as propriedades booleanas `autoload` e `minimal` para carregar diretamente a interface real em uma apresentação compacta. O leitor escolhe entre cinco abas de tarefas: conexão, atividade ao vivo, automações, pontos e sobreposições. Apenas o iframe da aba selecionada permanece ativo; a página não usa `ScenePreview` nem uma imitação visual do produto. Para criar uma incorporação com o mesmo comportamento, use:

```vue
<DemoFrame scenario="connection" locale="pt" panel="connection" title="Experimente o TikTools" autoload minimal />
```

Cada iframe da documentação mantém uma largura interna fixa de desktop de `1280` pixels CSS. O componente pai aplica a escala por transformação CSS fora do iframe; assim, as consultas de mídia internas continuam usando o layout de desktop. O ajuste automático é o padrão. O leitor também pode escolher `25%`, `50%`, `75%` ou `100%`; mudar o zoom preserva o estado e não recarrega a demonstração.

Para definir uma escala inicial explícita, passe a propriedade numérica `:scale="0.5"`. Use `viewportHeight` para ajustar a altura interna do quadro, mantendo a largura de desktop fixa:

```vue
<DemoFrame scenario="connection" locale="pt" panel="connection" title="Pratique sua primeira conexão" :scale="0.5" :viewport-height="720" />
```

O componente reserva a altura correspondente antes de o aplicativo terminar de carregar para evitar deslocamentos de conteúdo. Cada exemplo mantém sua instância de armazenamento isolada, e o controle de reinício limpa somente essa instância. O link para abrir a demonstração completa sai do modo incorporado e permite explorar o workspace. Mantenha as instruções e o resultado esperado próximos ao exemplo, usando `DocSteps` para sequências e `DocCallout` para orientações contextuais.

## Recuperação e verificação {#recovery-and-verification}

Redefina um cenário após uma lacuna ou falha simulada intencionalmente. Se ele contatar um serviço externo, pare e corrija o limite do transporte. Verifique conteúdo, compilação da demonstração e montagem do site independentemente; uma compilação de documentação não comprova correção nativa. [Testes](testing.md) distingue evidências de host simulado, navegador, Rust e aceitação manual.
