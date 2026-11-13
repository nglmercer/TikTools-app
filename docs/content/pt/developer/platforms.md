---
title: "Plataformas e compilações nativas para Termux"
docId: "developer.platforms"
translationKey: "developer.platforms"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "84f0988ac1aa15def527c00d0a134fb6c5c3bd00ef60efc7a376faec426a162f"
translationRevision: "84f0988ac1aa15def527c00d0a134fb6c5c3bd00ef60efc7a376faec426a162f"
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
