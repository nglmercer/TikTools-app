---
title: "Instale no Windows ou Linux"
docId: "user.start.installation"
translationKey: "user.start.installation"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "ed9e7c1bcb88d991ed3d8ea8a399aa5b5e9ca8cfa58de580f8ca757344d26ca2"
translationRevision: "a445ee7d830eb698c0e1fd2a069b89ecdf5839220578f1ac5875d49650d34634"
order: 20
---

# Instale no Windows ou Linux {#install-windows-or-linux}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Instale um pacote de desktop publicado sem configurar um compilador. Confira os [requisitos](requirements.md), feche qualquer instância aberta do TikTools e baixe das [versões no GitHub](https://github.com/nglmercer/TikTools-app/releases) do projeto.

<DocRequirements title="Antes de instalar">

- Um computador Windows ou Linux compatível com os [requisitos](requirements.md).
- Um pacote publicado para seu sistema, acompanhado do arquivo `SHA256SUMS.txt` da mesma versão.
- Todas as instâncias do TikTools fechadas. Se você já usa o aplicativo, faça uma cópia do diretório de dados com ele parado antes de trocar de versão.

</DocRequirements>

## Escolha um pacote {#choose-a-package}

<DocTabs label="Escolha seu sistema operacional">

<template #windows>

- **`installer.exe`:** instalador NSIS por usuário para Windows; exige WebView2.
- **ZIP portátil:** diretório com o executável e seus recursos. Extraia o pacote inteiro e mantenha os recursos junto ao EXE.

[Siga as instruções para Windows](#windows).

</template>

<template #linux>

- **`installer.AppImage`:** imagem executável portátil para um desktop Linux compatível.
- **`installer.deb`:** instalação pelo gerenciador de pacotes Debian/Ubuntu.
- **TAR.GZ portátil:** diretório com o executável e seus recursos. Extraia o pacote inteiro para conservar os arquivos necessários.

[Siga as instruções para Linux](#linux).

</template>

</DocTabs>

Verifique se a versão é estável ou preliminar. Versões preliminares são candidatas para depuração, não promessas automáticas de compatibilidade em produção. Use o `SHA256SUMS.txt` da versão para conferir os bytes baixados antes de substituir uma instalação existente.

## Windows {#windows}

<DocSteps>

1. Baixe o instalador x86_64 e verifique sua soma de verificação.
2. Execute o instalador na conta de usuário que usará o TikTools. Instale o WebView2 se solicitado.
3. Inicie o TikTools. Abra **Configurações → Geral** e use **Idioma da interface** para selecionar o idioma desejado.
4. Para um ZIP portátil, extraia o diretório inteiro. Mantenha `web/` e os outros recursos do pacote ao lado do executável; não execute apenas uma cópia do EXE.

</DocSteps>

## Linux {#linux}

<DocSteps>

1. Baixe o AppImage, DEB ou arquivo portátil correspondente e verifique sua soma de verificação.
2. Para AppImage, permita a execução no gerenciador de arquivos ou execute `chmod +x` no arquivo baixado e depois abra-o.
3. Para DEB, instale pelo gerenciador de pacotes do sistema para resolver as dependências de execução.
4. Para TAR.GZ, extraia o diretório completo e inicie seu executável de desktop com os recursos do pacote intactos.

</DocSteps>

## Resultado esperado {#expected-result}

O TikTools abre offline. Os dados ficam separados da instalação, normalmente em `%LOCALAPPDATA%/TikTools` no Windows ou `~/.local/share/TikTools` no Linux.

## Recuperação {#recovery}

Se o pacote falhar antes de mostrar uma janela, verifique os diagnósticos e erros de bibliotecas ausentes antes de modificar configurações de conta ou migração. Faça backup do diretório de dados com o aplicativo parado antes de tentar outra versão. Reinstalar um executável antigo não reverte migrações do banco de dados. Continue com [sua primeira LIVE](first-live.md) ou com as [atualizações](updates.md).

## Detalhes de operação {#operational-details}

Se a janela não abrir, procure o log nativo `tiktools.log` (`controller.log` no modo controlador): Windows `%LOCALAPPDATA%/TikTools/logs`, Linux `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs`, salvo alterações em `TIKTOOLS_HOME` ou `TIKTOOLS_LOG_DIR`. O registro pode falhar antes de criar o arquivo. Preserve o erro do terminal e confira [retenção e privacidade](preferences/troubleshooting.md#operational-details) antes de compartilhar.
