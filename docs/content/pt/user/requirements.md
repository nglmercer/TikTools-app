---
title: "Requisitos"
docId: "user.start.requirements"
translationKey: "user.start.requirements"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f3bc30b94983b95c7ce6ffc7fe8b3707aa175cadd2156f26f16b8b77be83ea6b"
translationRevision: "f3bc30b94983b95c7ce6ffc7fe8b3707aa175cadd2156f26f16b8b77be83ea6b"
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
