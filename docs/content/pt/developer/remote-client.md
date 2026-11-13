---
title: "Clientes remotos e recuperação"
docId: "developer.remote-client"
translationKey: "developer.remote-client"
locale: "pt"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a6a7a67da0a7349c2c8fd860de9d945e841ee1f1a82057c312f4cc193fbd9fe8"
translationRevision: "a6a7a67da0a7349c2c8fd860de9d945e841ee1f1a82057c312f4cc193fbd9fe8"
order: 90
---

# Clientes remotos e recuperação {#remote-clients-and-recovery}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Conecte um controlador a um host sem criar um núcleo local de LIVE. Você precisa de um host com os recursos remotos relevantes ativos, convite aprovado pelo proprietário, origem HTTPS acessível e armazenamento seguro de credenciais compatível. Leia [colaboração](collaboration.md) para concessões e [transportes](transports.md) para opções de rota nativa.

## Pareie e inspecione {#pair-and-inspect}

Estes comandos descrevem um ambiente de teste manual; substitua a origem de exemplo pelo host configurado:

```sh
tiktools remote inspect --url https://host.example
tiktools remote pair --url https://host.example --profile live-desk
tiktools --remote-profile live-desk api discover
tiktools --remote-profile live-desk live status
tiktools --remote-profile live-desk workflow list
```

Para um host com certificado autoassinado, compare sua impressão digital com o proprietário por um canal independente e passe `--fingerprint` durante o pareamento. A inspeção não envia uma requisição de negócio autenticada nem aprova confiança. Valores de convite e credenciais são inseridos pelos prompts compatíveis, não como segredos na linha de comando.

Arquivos de perfil contêm identidade, política de confiança e referência opaca ao armazenamento seguro. Linux usa Secret Service, macOS Keychain e Windows Credential Manager. Um armazenamento indisponível falha explicitamente; Android não tem backend selecionado para perfis persistentes. O SDK pode usar credenciais em memória fornecidas explicitamente. `remote forget` remove credenciais e metadados locais, mas não revoga a sessão do host.

## Preserve a identidade do comando {#preserve-command-identity}

Leia um recurso com metadados antes de uma mutação compatível. Guarde sua revisão base, instância do host e ID durável de comando. Se a conexão desaparecer após o envio, consulte `collaboration.commands.get` com o ID original. Um recibo é histórico; atualize o recurso separadamente. Desativar/excluir fluxos, ajustar pontos e rascunhos de automação desativados sob autoria do criador têm contratos e concessões diferentes.

## Resultado esperado e status {#expected-result-and-status}

O desktop nativo tem uma entrada isolada `tiktools-desktop --controller` e diários vinculados ao perfil. A cobertura do controlador continua experimental. Pareamento, conexão de transporte ou uma flag ampla de recurso não autorizam edições arbitrárias. Leituras remotas de automações expõem resumos fechados, nunca código editável; ativação/disparo remoto e adoção pelo proprietário local continuam separados.

## Recupere {#recover}

Em uma lacuna de sequência, use o caminho negociado de instantâneo/retomada da assinatura e limpe projeções privadas quando a identidade mudar. Não reproduza mutações de negócio automaticamente após reconectar. Um certificado alterado exige revisão explícita de confiança. A [implementação de perfis](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/remote/profiles/mod.rs) documenta as concessões temporárias de perfil e a incerteza de renovação; a aceitação real entre vários PCs e armazenamentos do sistema deve ser informada separadamente dos testes de loopback.
