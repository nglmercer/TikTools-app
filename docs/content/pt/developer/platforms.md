---
title: "Plataformas e compilações nativas para Termux"
docId: "developer.platforms"
translationKey: "developer.platforms"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "c7befe2be6f91bcf50ce4572aaa95d02723a315ba11ed1d94e6d499e802c2473"
translationRevision: "249b9e9d619625c2014e6c44ed080e39cc22721483ee01f3f9e5a5c3fa829def"
order: 20
---

# Plataformas e compilações nativas para Termux {#platforms-and-native-termux-builds}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Escolha uma compilação correspondente ao sistema operacional de destino. Desenvolvimento de desktop precisa de Bun 1.4.4, Rust 1.95 ou mais recente e das dependências de WebView da plataforma descritas na [configuração](setup.md). Termux compila a CLI Rust diretamente no diretório privado do Android; armazenamento compartilhado do Android é inadequado para executáveis e sockets Unix.

## Compile o host adequado {#build-the-appropriate-host}

Windows usa MSVC Build Tools e WebView2. Strawberry Perl nativo dá suporte ao OpenSSL incorporado; não o substitua por Perl do MSYS. O desktop Linux precisa de GTK3, WebKitGTK 4.1, bibliotecas de desenvolvimento de áudio e `pkg-config`. A integração Winit/Wry pertence a `crates/tiktools-desktop/src/platform.rs`; serviços de negócio do núcleo continuam independentes das ferramentas de janela.

Em um telefone ARM64, primeiro verifique se `rustc -vV` informa `aarch64-linux-android`. No Termux:

```sh
pkg install git rust clang make cmake pkg-config python protobuf ca-certificates
export CARGO_BUILD_JOBS=2
bash scripts/termux/check.sh
cargo build -p tiktools-cli --locked
./target/debug/tiktools --standalone system info
```

Cargo é intencional aqui: o fluxo nativo no telefone não exige Bun nem Node. Em um desktop com Bun, encaminhe Cargo por `bun scripts/task.ts rust ...` para manter o tratamento de vinculador/cache. Para dois terminais no telefone, inicie `./target/debug/tiktools host --ipc` e use o mesmo `TIKTOOLS_HOME` nas chamadas CLI seguintes.

## Resultado esperado e status {#expected-result-and-status}

IPC local, SQLite e plugins portáteis de processo/QuickJS têm caminhos Android implementados. Artefatos de processo devem ter Android/Bionic como destino. Bibliotecas nativas e addons `.node` estão indisponíveis no Android. Áudio, atalhos globais, notificações, seletores de arquivo e WebViews de desktop não têm backend no telefone. Perfis remotos persistidos falham explicitamente porque Android não tem backend selecionado de armazenamento de credenciais; credenciais do SDK podem permanecer na memória.

## Verifique e recupere {#verify-and-recover}

Execute `bash scripts/termux/doctor.sh`, `bash scripts/termux/test.sh` e `bash scripts/termux/smoke.sh` da raiz do checkout. Informe honestamente verificações de frontend ignoradas. Para símbolos C++ quebrados de `protoc`, compare `env -u LD_LIBRARY_PATH protoc --version` antes de alterar bibliotecas. Um pacote de certificados CA ausente deve ser corrigido, nunca contornado. Compilação cruzada sozinha não comprova aceitação no telefone. O [script smoke do Termux](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/termux/smoke.sh) registra as sondagens específicas e restrições restantes.

## Detalhes de operação {#operational-details}

Para navegador local, construa assets com index.html e CLI com `local-web`. Remova overrides herdados e use `TIKTOOLS_HOME` descartável; inicie primeiro host IPC. Em outro terminal com a mesma raiz execute o adaptador, abra link loopback ou digite código de seis dígitos de um uso (cinco minutos). Pare adaptador e host separadamente. Loopback/pairing não oferecem acesso remoto/LAN. Termux exige toolchain Android nativo e assets web; binário Linux desktop não é Android.

```sh
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-cli --features local-web --locked
export TIKTOOLS_HOME=/absolute/path/disposable-home
target/debug/tiktools host --ipc
```

```sh
export TIKTOOLS_HOME=/absolute/path/disposable-home
target/debug/tiktools web --assets /absolute/path/to/TikTools/dist/web --port 4174
```
