---
title: "Downloads e atualizações"
docId: "user.start.updates"
translationKey: "user.start.updates"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "59d259c554c7b220bb7e978e2da3165034eeb494aa28e5c73666be9945fb6869"
translationRevision: "59d259c554c7b220bb7e978e2da3165034eeb494aa28e5c73666be9945fb6869"
order: 40
---

# Downloads e atualizações {#downloads-and-updates}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Escolha o canal de versão correto e atualize sem confundir uma verificação de atualização com instalação automática. Feche sessões ativas e faça backup dos dados locais antes de mudar de versão, especialmente quando houver uma migração de banco de dados documentada.

## Selecione um canal {#select-a-channel}

A [lista de versões](https://github.com/nglmercer/TikTools-app/releases) identifica versões estáveis e preliminares. Uma candidata como `-rc` se destina à verificação. O atualizador do aplicativo distingue feeds estável e beta; ele autentica metadados assinados antes de oferecer uma versão. Um arquivo JSON sem assinatura ou um instalador alterado não é uma atualização aceita.

## Verifique e instale {#check-and-install}

1. Abra **Settings → General** e inspecione o estado atual do atualizador e a versão instalada.
2. Execute a verificação disponível. Compilações sem a chave pública ou capacidade de feed configurada mostram que as verificações estão indisponíveis.
3. Leia as notas de versão e escolha o pacote para sua plataforma.
4. Quando a compilação oferecer download ou encaminhamento manual, siga esse caminho. O fluxo de lançamento de produção habilita verificações autenticadas; ele não habilita todos os recursos experimentais de instalação automática.
5. Para uma atualização manual, feche o TikTools, substitua ou instale o pacote verificado completo e reabra. Mantenha o diretório de dados separado dos arquivos do aplicativo.

## Resultado esperado {#expected-result}

O aplicativo reiniciado informa a versão escolhida e consegue ler o perfil local esperado. Pacotes portáteis ZIP/TAR exigem substituição manual. Instalações com disposição desconhecida ou extraída também podem exigir um caminho manual mesmo quando existe um instalador AppImage.

## Recuperação e limitações {#recovery-and-limitations}

Uma falha na verificação de assinatura significa falha na atualização; baixe uma cópia oficial nova e verifique as somas de verificação da versão. Não contorne a verificação. A substituição automática de AppImage e o encaminhamento para NSIS têm controles de compilação separados e são experimentais. Não existe reversão automática geral. Voltar a um binário antigo não desfaz um esquema SQLite migrado.

Informe a versão instalada, o formato do pacote e o erro do atualizador em Diagnostics. Não anexe chaves de assinatura, tokens de convite ou dados da conta. O [guia de instalação](installation.md) descreve as disposições de pacote compatíveis.
