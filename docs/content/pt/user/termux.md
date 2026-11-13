---
title: "Termux e operação sem interface gráfica"
docId: "user.start.termux"
translationKey: "user.start.termux"
locale: "pt"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "12f13cdca1cf24f7de30d83fa8f3844f9d1beee6c910c4deb9be817acb89a009"
translationRevision: "12f13cdca1cf24f7de30d83fa8f3844f9d1beee6c910c4deb9be817acb89a009"
order: 50
---

# Termux e operação sem interface gráfica {#termux-and-headless-scope}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Entenda quais partes do TikTools funcionam sem uma WebView de desktop. Este é um fluxo avançado, não uma versão Android do instalador Windows/Linux. O projeto tem uma CLI Rust nativa para Termux e um host sem interface gráfica; obter um binário correspondente ou compilá-lo faz parte da configuração de desenvolvimento.

## Use um único host {#use-one-host}

1. Inicie um host sem interface gráfica com `tiktools host --ipc` no ambiente onde a CLI foi instalada.
2. Mantenha o mesmo diretório de dados do aplicativo entre os terminais.
3. Em outro terminal, use `tiktools system info`, `tiktools live status` ou `tiktools api discover` para inspecionar esse host.
4. Pare-o corretamente com o comando de encerramento documentado ou Ctrl-C. Não remova os bloqueios de propriedade enquanto ele estiver em execução.

Comandos comuns da CLI se conectam ao host existente. `--standalone` cria explicitamente um núcleo separado para uma invocação; não é a maneira padrão de controlar um host já em execução.

## Prévia no navegador local {#local-browser-preview}

O adaptador opcional `local-web` pode servir a interface Vue para um navegador após o pareamento local. É uma prévia de desenvolvimento com requisitos separados de compilação e ciclo de vida. Ele não torna o Android um destino compatível de WebView de desktop nem concede acesso ao host a uma pessoa remota.

## Resultado esperado {#expected-result}

A CLI e o desktop compartilham conceitos de controle, persistência SQLite e processamento nativo de eventos. Plugins dependentes de plataforma, áudio, atalhos globais, janelas nativas e armazenamentos de credenciais podem estar indisponíveis. A persistência nativa de perfis remotos atualmente tem restrições explícitas no Android.

## Recuperação {#recovery}

Use o diretório privado do aplicativo Termux em vez do armazenamento compartilhado do Android, que pode impedir execução e sockets Unix. Mantenha o mesmo diretório de dados do aplicativo e confirme que o host está ativo antes de depurar uma falha de IPC. Se o Android encerrar o processo, reinicie-o com o mesmo perfil e inspecione os diagnósticos de recuperação. Não espere que a demonstração da documentação reproduza o comportamento de ciclo de vida do Android.

O [guia de desenvolvimento](../developer/index.md) contém as ferramentas, flags de recursos e procedimentos de verificação no dispositivo. A instalação de binários de desktop continua no [guia Windows/Linux](installation.md).
