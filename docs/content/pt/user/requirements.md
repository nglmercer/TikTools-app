---
title: "Requisitos"
docId: "user.start.requirements"
translationKey: "user.start.requirements"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "c68c149552a943cf0f03393c5dac91bd62abf9c50f791d7b0757599754f4804e"
translationRevision: "7592985d7d424b5727216c61f7663cc72f9a0abca1e6aa506b7d99a21afa22ff"
order: 10
---

# Requisitos {#requirements}

## Objetivo {#goal}

Escolha um pacote de desktop compatível e identifique os serviços necessários para o fluxo desejado antes de instalar.

## O que você precisa {#what-you-need}

- Um computador Windows x86_64 ou Linux x86_64 para os pacotes de desktop publicados. O fluxo de lançamento atual compila esses dois destinos. Ele não publica um desktop macOS nem um APK Android.
- Uma sessão gráfica funcional. O Windows precisa do WebView2 Runtime da Microsoft. O Linux precisa de um ambiente GTK/WebKitGTK compatível; o DEB declara seus pacotes de execução e o AppImage inclui auxiliares de mídia revisados.
- Acesso à rede do TikTok para uma conexão real com uma LIVE. Uma sala offline, sessão de visitante expirada ou limite de requisições do serviço não se corrige alterando uma automação.
- Armazenamento fora do diretório de instalação para dados SQLite, plugins e registros. O aplicativo determina um diretório de dados por usuário; mover o executável não move esses dados.

Uma conta do Cloud é necessária para operações de catálogo ou colaboração baseadas em conta, não para todos os editores locais. O OBS só é necessário se você quiser exibir um widget em uma transmissão. Integrações de áudio/TTS podem exigir um plugin instalado separadamente e seu próprio provedor ou serviço.

## Verifique o ambiente {#check-the-environment}

1. Selecione o pacote correspondente ao seu sistema operacional e à arquitetura x86_64.
2. No Windows, instale o WebView2 se o instalador informar sua ausência. No Linux, resolva bibliotecas de execução ausentes pelo gerenciador de pacotes da distribuição.
3. Confirme que o aplicativo abre sua janela antes de conectar ao TikTok ou adicionar plugins.
4. Continue com a [instalação](installation.md) e depois com [sua primeira LIVE](first-live.md).

## Resultado esperado {#expected-result}

O aplicativo abre e a navegação principal funciona mesmo offline. Integrações opcionais podem continuar indisponíveis até que seu plugin ou a capacidade do host esteja presente.

## Recuperação {#recovery}

Para uma janela em branco ou encerramento precoce, colete os diagnósticos de inicialização e a versão exata do pacote. AppImage e DEB são formatos de instalação diferentes; nenhum é um executável Android. Consulte [Termux e operação sem interface gráfica](termux.md) para esse ambiente. A compilação do código-fonte tem requisitos adicionais no guia de desenvolvimento.

## Detalhes de operação {#operational-details}

Se a janela não abrir, procure o log nativo `tiktools.log` (`controller.log` no modo controlador): Windows `%LOCALAPPDATA%/TikTools/logs`, Linux `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs`, salvo alterações em `TIKTOOLS_HOME` ou `TIKTOOLS_LOG_DIR`. O registro pode falhar antes de criar o arquivo. Preserve o erro do terminal e confira [retenção e privacidade](preferences/troubleshooting.md#operational-details) antes de compartilhar.
