---
title: "Clientes remotos e recuperação"
docId: "developer.remote-client"
translationKey: "developer.remote-client"
locale: "pt"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "1f6c56c60e69e8804bf94b4b0b86311da33a0a65506c5d93501804a24abe4a1b"
translationRevision: "d9ba88031218b672a92a8591ac9774583689a90e36c5fff63094f2b90860d258"
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

## Detalhes de operação {#operational-details}

Aplique cada evento confiável à captura/projeção correspondente antes de `acknowledge`; confirmar antes perde evidências de recuperação. Retome apenas com a mesma instância, assinatura, sessão e permissão. Lacuna, contexto alterado, cursor rejeitado ou fechamento do navegador por gap exige novas capturas autorizadas e outra assinatura. Não repita mutações pelo histórico. Use a [receita de metadados/ID original](control-api.md#operational-details) para escritas incertas.

O `event` local contém `{topic,data?}`; variantes unitárias podem omitir dados. Em remoto `event.params` é ReliableEvent com sequência textual, ID, versão do recurso e ator. `event.gap` é outro método. Os exemplos são ilustrativos, não replay durável de efeitos perdidos.

```json
{"jsonrpc":"2.0","method":"event","params":{"topic":"live.status","data":{}}}
{"jsonrpc":"2.0","method":"event","params":{"topic":"workflow.changed","data":{},"hostInstanceId":"h1","subscriptionId":"s1","sequence":"7","eventId":"e7","resource":{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"},"actor":{"principalId":"owner"}}}
{"jsonrpc":"2.0","method":"event.gap","params":{"lost":3,"resync":true}}
```

O esboço SDK abaixo exige remote-profiles. Callbacks de projeção pertencem ao aplicativo e terminam antes de acknowledge. Veja [RecoveryStream](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/remote/recovery.rs) para baseline, cursor e retomada tipados.

```rust
use tiktools_client::remote::{RecoveryMode, RecoveryStream, StreamDomain};
use std::collections::BTreeSet;

let mut stream = RecoveryStream::start(remote, BTreeSet::from([StreamDomain::Workflows])).await?;
replace_snapshot(stream.baseline().expect("complete authorized baseline"))?;
loop {
    match stream.next_event().await {
        Ok(event) => {
            apply_to_projection(&event)?;
            stream.acknowledge(&event)?;
        }
        Err(_) => {
            if stream.reconnect().await? == RecoveryMode::Snapshot {
                replace_snapshot(stream.baseline().expect("fresh authorized baseline"))?;
            }
        }
    }
}
```

## Operações de linha de comando {#command-line-operations}

Cada operação deste guia também está disponível na CLI — offline, scriptável e apta para agentes. As linhas de uso são sintaxe do parser e permanecem em inglês em todos os idiomas.

<!--@include: ../../../.generated/cli-domain-web.pt.md-->

Consulte a [referência completa de comandos](cli.md) para as convenções, a saída JSON e os marcadores de operações destrutivas.
