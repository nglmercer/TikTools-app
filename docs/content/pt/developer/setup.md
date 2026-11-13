---
title: "Configuração e ciclos de desenvolvimento"
docId: "developer.setup"
translationKey: "developer.setup"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "8842472fa359a7c4b2e33bbd6e84a900b9a6dd71a8207afcf61aa507be7494d3"
translationRevision: "8842472fa359a7c4b2e33bbd6e84a900b9a6dd71a8207afcf61aa507be7494d3"
order: 10
---

# Configuração e ciclos de desenvolvimento {#development-setup-and-loops}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Compile a partir do código atual de `remake` sem alterar o perfil instalado do usuário. Use Bun 1.4.4 de `package.json`, Rust 1.95 ou mais recente e Node compatível com o Vite travado: `^20.19.0 || >=22.12.0`. A CI atualmente usa Rust 1.99.1. Compilar o código-fonte é separado da [instalação de binários](../user/installation.md).

## Configuração da plataforma {#platform-setup}

No Windows, instale MSVC Build Tools, WebView2 e Strawberry Perl nativo para OpenSSL incorporado. O wrapper do Cargo descobre o ambiente MSVC e o Perl nativo, preservando substituições explícitas de `OPENSSL_SRC_PERL`/`PERL`. Perl de MSYS/Cygwin é inadequado para esse caminho MSVC. O `lld-link` do LLVM é opcional.

Para desenvolvimento de desktop no Ubuntu 24.04, o conjunto de dependências da CI inclui `libasound2-dev`, `libappindicator3-dev`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libxdo-dev` e `pkg-config`. O empacotamento AppImage também precisa dos metadados de desenvolvimento do GStreamer, dos plugins base/good/libav e de seu scanner. O `mold` opcional acelera a vinculação no Linux. Android/Termux usa ferramentas Android nativas em vez dessas bibliotecas de desktop.

## Instale dependências {#install-dependencies}

```sh
git switch remake
bun ci
```

Use os lockfiles Bun e Cargo versionados. `bun install` é para alterações intencionais de dependências. A documentação é um workspace explícito: seu VitePress estável tem uma dependência Vite isolada, enquanto o aplicativo mantém Vite 8. Não force ambos a usar a mesma versão principal do Vite.

## Escolha um ciclo {#choose-a-loop}

```sh
bun run start                 # Vite gerenciado + host Wry de desenvolvimento desktop
bun run dev                   # desenvolvimento nativo no navegador local, sem preparar plugins de desenvolvimento
bun scripts/task.ts serve     # apenas Vite; exige host/adaptador real para operações nativas
bun scripts/task.ts packaged  # caminho do frontend empacotado, em vez de uma URL de desenvolvimento
```

`start` inicia o Vite, aguarda sua URL alocada e executa o desktop com `TIKTOOLS_DEV_URL`. Ele pode preparar plugins do checkout separado `plugins-example-tiktools`, encontrado por `TIKTOOLS_PLUGINS_REPO` ou pelo diretório irmão. Ele não compila implementações de plugins versionadas neste repositório. Use `TIKTOOLS_SKIP_DEV_PLUGINS=1` para omitir essa preparação. Nenhum host é substituído automaticamente; um proprietário de desktop existente causa erro de inicialização.

Para experimentos nativos isolados, defina `TIKTOOLS_HOME` para um diretório descartável antes de iniciar o host. Use o mesmo diretório em todos os terminais CLI que controlam esse host. Nunca exclua um bloqueio de propriedade ativo.

## Valide e recupere {#validate-and-recover}

```sh
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts rust fmt --all -- --check
bun scripts/task.ts rust-lint
bun scripts/task.ts rust test --workspace --locked -- --test-threads=1
```

Execute testes específicos durante a iteração e mantenha todas as verificações obrigatórias de CI antes de mesclar. Não execute várias compilações Cargo no mesmo diretório de destino enquanto mede testes de assinatura que invocam Cargo. Se uma inicialização recusar um host antigo, pare corretamente o processo proprietário; se faltar um vinculador ou dependência de WebView, corrija a configuração da plataforma antes de alterar a lógica do aplicativo.
