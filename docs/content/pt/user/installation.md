---
title: "Instale no Windows ou Linux"
docId: "user.start.installation"
translationKey: "user.start.installation"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "555ad595d29dac6659c260a8f2d99acc65bdf8bba49ca87d1517f527ab15469d"
translationRevision: "555ad595d29dac6659c260a8f2d99acc65bdf8bba49ca87d1517f527ab15469d"
order: 20
---

# Instale no Windows ou Linux {#install-windows-or-linux}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Instale um pacote de desktop publicado sem configurar um compilador. Confira os [requisitos](requirements.md), feche qualquer instância aberta do TikTools e baixe das [versões no GitHub](https://github.com/nglmercer/TikTools-app/releases) do projeto.

## Escolha um pacote {#choose-a-package}

| Pacote | Uso |
| --- | --- |
| Windows `installer.exe` | Instalação NSIS por usuário; exige WebView2. |
| Linux `installer.AppImage` | Imagem executável portátil para um desktop Linux compatível. |
| Linux `installer.deb` | Instalação pelo gerenciador de pacotes Debian/Ubuntu. |
| Windows ZIP / Linux TAR.GZ | Diretório portátil com o executável e seus recursos. |

Verifique se a versão é estável ou preliminar. Versões preliminares são candidatas para depuração, não promessas automáticas de compatibilidade em produção. Use o `SHA256SUMS.txt` da versão para conferir os bytes baixados antes de substituir uma instalação existente.

## Windows {#windows}

1. Baixe o instalador x86_64 e verifique sua soma de verificação.
2. Execute o instalador na conta de usuário que usará o TikTools. Instale o WebView2 se solicitado.
3. Inicie o TikTools. Abra **Settings → General** para selecionar o idioma da interface.
4. Para um ZIP portátil, extraia o diretório inteiro. Mantenha `web/` e os outros recursos do pacote ao lado do executável; não execute apenas uma cópia do EXE.

## Linux {#linux}

1. Baixe o AppImage, DEB ou arquivo portátil correspondente e verifique sua soma de verificação.
2. Para AppImage, permita a execução no gerenciador de arquivos ou execute `chmod +x` no arquivo baixado e depois abra-o.
3. Para DEB, instale pelo gerenciador de pacotes do sistema para resolver as dependências de execução.
4. Para TAR.GZ, extraia o diretório completo e inicie seu executável de desktop com os recursos do pacote intactos.

## Resultado esperado {#expected-result}

O TikTools abre offline. Os dados ficam separados da instalação, normalmente em `%LOCALAPPDATA%/TikTools` no Windows ou `~/.local/share/TikTools` no Linux.

## Recuperação {#recovery}

Se o pacote falhar antes de mostrar uma janela, verifique os diagnósticos e erros de bibliotecas ausentes antes de modificar configurações de conta ou migração. Faça backup do diretório de dados com o aplicativo parado antes de tentar outra versão. Reinstalar um executável antigo não reverte migrações do banco de dados. Continue com [sua primeira LIVE](first-live.md) ou com as [atualizações](updates.md).
