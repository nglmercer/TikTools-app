---
title: "Testes e limites das evidências"
docId: "developer.testing"
translationKey: "developer.testing"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "455bcdbf94d1bff28bd50dcb993cdff59dd000214e25c49bb194c9b3124e8e59"
translationRevision: "455bcdbf94d1bff28bd50dcb993cdff59dd000214e25c49bb194c9b3124e8e59"
order: 210
---

# Testes e limites das evidências {#tests-and-evidence-boundaries}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Escolha verificações que possam comprovar o comportamento alterado. Use Bun 1.4.4, uma instalação fiel ao lockfile e Rust apenas para os limites nativos afetados. Mantenha testes isolados de hosts, credenciais e contas de produção. Um cenário de navegador bem-sucedido é evidência útil para seu limite, não para toda integração abaixo dele.

## Escolha uma suíte focada {#choose-a-focused-suite}

```sh
bun scripts/task.ts test
bun scripts/task.ts test web control-client
bun scripts/task.ts test bun automation
bun scripts/task.ts test rust tiktools-core batched_likes --features persistence,native-tiktok
bun scripts/task.ts rust test -p tiktools-client --test parity --locked
bun scripts/task.ts contracts --check
```

A suíte rápida padrão combina testes unitários Bun e suítes Vitest web/widgets. O executor específico valida argumentos e rejeita filtros ausentes de testes Rust em vez de aceitar zero testes correspondentes. Use o repasse `rust` quando precisar de flags de alvo Cargo além da gramática do executor específico.

## Verifique o limite real {#verify-the-actual-boundary}

Testes de componentes Vue usam o cliente de controle simulado compartilhado. Os testes principais Playwright exercitam a mesma SPA com um host `window.ipc` simulado instalado antes da navegação. RPCs desconhecidos falham claramente e erros de console/página são verificados. Eles executam Chromium, não Wry. Suítes de navegador de interface de plugins ficam no repositório externo de plugins. Suítes do gateway de widgets testam o protocolo/cliente compartilhado e recursos compilados.

Para trabalho autorizado no navegador após compilar recursos, use `bun scripts/task.ts test e2e`. Isso abre um navegador, portanto é intencionalmente separado de edição de documentação e revisão de código sem interface gráfica. Testes do shell nativo, fonte de navegador do OBS, telefone, armazenamento de credenciais do sistema, substituição pelo atualizador e dois PCs precisam de seus ambientes reais e evidências explícitas.

## Tempo de execução e custo de CI {#execution-time-and-ci-cost}

Comece localmente com um filtro de arquivo/nome ou `bun scripts/task.ts test --changed`. Uma execução completa de frontend mantém todos os arquivos isolados; Vitest usa `vmThreads` para reutilizar o runtime DOM por worker em vez de compartilhar estado do aplicativo entre arquivos.

A CI compara o intervalo completo do push ou pull request. Caminhos de documentação revisados omitem tarefas nativas; alterações Vue/demo executam a verificação de frontend. Código nativo, infraestrutura de compilação, manifestos de dependências, caminhos desconhecidos ou comparação indisponível mantêm todas as verificações existentes. `scripts/ci-scope.mjs` é a política de seleção testada. CI manual oferece `frontend`, `native` ou `full`; chamadores reutilizáveis usam validação completa por padrão. O status final da CI rejeita tarefas selecionadas que falharam ou foram canceladas.

Caches Rust são separados por papel, sistema operacional, ferramentas, lockfile e revisão de código. A CI mantém informações de depuração de tabelas de linhas e desativa sessões incrementais para reduzir o volume de cache. Testes seriais de SQLite/perfil continuam seriais até seus recursos compartilhados serem isolados independentemente. Não remova essa proteção para obter uma medição mais rápida.

O fluxo do site é manual, portanto editar ou enviar documentação não inicia uma segunda compilação remota. A recuperação de lançamento também é manual: seu padrão verifica apenas Linux e não publica. As verificações de ambas as plataformas e uma escolha explícita de publicar são exigidas para a publicação de uma versão. Nunca dispare nem repita trabalho em executores pagos apenas para confirmar uma edição de Markdown.

## Resultado esperado e status {#expected-result-and-status}

Um relatório de alteração nomeia a suíte aprovada e seu escopo. Execute verificações de lint/tipagem/compilação de frontend e os controles obrigatórios de CI para alterações de implementação; conteúdo de documentação usa `bun scripts/task.ts docs check` e `docs test`. Não altere dependências apenas para fazer um comando de teste baixar uma ferramenta mais recente.

## Recuperação {#recovery}

Se um filtro não corresponder a nada, corrija-o em vez de informar sucesso. Se um cenário de teste não tiver um RPC recém-exigido, amplie seu contrato explícito em vez de fazer chamadas desconhecidas terem sucesso. Mantenha visível a aceitação manual incerta/ignorada. O [executor de testes](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/lib/test-runner.ts) define a sintaxe atual de comandos; [desempenho](performance.md) explica evidências de carga reproduzíveis.
