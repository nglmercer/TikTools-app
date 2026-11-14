---
title: "Entre na sua conta do Cloud"
docId: "user.cloud.account"
translationKey: "user.cloud.account"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "02b7718e7fc7b2a53b68ee1c6e19e622608a6fdfc7eac553879c474ca69ea74b"
translationRevision: "78ce7c124fefac65fc37718d87fb15fd386130fc9a16414febfc481f33749dba"
order: 10
---

# Entre na sua conta do Cloud {#sign-in-to-your-cloud-account}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Conecte o aplicativo de desktop ao serviço Cloud configurado para operações de catálogo e colaboração baseadas em conta. Você precisa de acesso à rede, um navegador do sistema e um armazenamento de credenciais do sistema operacional funcional. A edição local continua útil sem o Cloud; uma sessão do Cloud não autentica sua conexão com a LIVE do TikTok.

## Entre e verifique a identidade {#sign-in-and-verify-identity}

1. Abra **Settings → Account**. Leia o estado atual antes de iniciar outro login.
2. Selecione **Sign in**. O desktop inicia um receptor local de retorno e abre a página de login configurada no navegador do sistema.
3. Conclua o fluxo de login do serviço e retorne ao TikTools. Se o navegador não abrir, use a URL de login mostrada pelo aplicativo para esta tentativa. Trate essa URL como privada e não a publique.
4. Aguarde o estado da conta indicar que a sessão está iniciada. Confirme o nome ou e-mail exibido antes de abrir um catálogo privado ou equipe.
5. Expanda os diagnósticos de Account se o status informar um erro. Use **Cancel** para abandonar uma tentativa inacabada em vez de deixar várias tentativas ativas.
6. Ao terminar em um computador compartilhado, selecione **Sign out** e verifique se o estado da conta mudou.

O host nativo gerencia o token da sessão. Ele armazena credenciais pelo armazenamento de credenciais do sistema operacional e mantém apenas status não secreto no frontend. Não existe alternativa em arquivo de texto simples quando o armazenamento seguro está indisponível. Uma pasta de dados copiada não é uma sessão autenticada portátil.

## Resultado esperado e status {#expected-result-and-status}

Login, restauração e saída nativos estão disponíveis em hosts compilados com autenticação do Cloud. Uma conta configurada pode ser restaurada ao reiniciar e revalidada com o serviço. Um host não configurado ou incompatível não se corrige fornecendo um cookie diferente do TikTok.

A disponibilidade do login do serviço e a aceitação real por provedor/plataforma dependem do serviço implantado e da sua máquina. A validação local não comprova que todas as rotas de login de produção funcionam.

## Recuperação {#recovery}

Para uma tentativa cancelada ou expirada, inicie um login novo após fechar o anterior. Para erros de rede ou serviço, preserve o código de diagnóstico e tente novamente após a recuperação. Para um erro do armazenamento de credenciais, restaure a disponibilidade do armazenamento seguro do sistema operacional; não cole um token de portador nas configurações nem em uma mensagem de suporte. Consulte [Changes](changes.md) para sincronização explícita e [colaboração](collaboration.md) para concessões de acesso separadas.

## Detalhes de operação {#operational-details}

Sair apaga credenciais locais e limpa o contexto antes de tentar revogação remota de melhor esforço. A interface desconectada confirma remoção local, não revogação remota durante falha de rede. Use administração de sessões do serviço se disponível. Exporte Diagnostics antes de sair/trocar conta conhecida: esse histórico UI é apagado. Cookies TikTok e sessões de host remoto são identidades separadas.
