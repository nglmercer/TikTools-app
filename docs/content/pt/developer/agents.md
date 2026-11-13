---
title: "Agentes e o SDK Rust tipado"
docId: "developer.agents"
translationKey: "developer.agents"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7c0b540c4f1b4e1953b3007092ce5440109f355d8ad06eb763aa60d3616f23d0"
translationRevision: "7c0b540c4f1b4e1953b3007092ce5440109f355d8ad06eb763aa60d3616f23d0"
order: 80
---

# Agentes e o SDK Rust tipado {#agents-and-the-typed-rust-sdk}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Crie scripts ou agentes para o mesmo host usado pelo desktop. Tenha um host isolado em execução, uma CLI `tiktools` correspondente e acesso à [descoberta](control-api.md). O SDK fica em `crates/tiktools-client`; ele expõe backends direto, local conectado e remoto opcional sem introduzir outro núcleo de negócio nos caminhos comuns de cliente.

## Comece pela descoberta somente de leitura {#start-with-read-only-discovery}

```sh
target/debug/tiktools --json system info
target/debug/tiktools api discover
target/debug/tiktools api schema live.status
target/debug/tiktools api call live.status '{}' --format json
```

Determine as capacidades disponíveis e os esquemas de parâmetros antes de gerar uma ação. Não deduza conexão com TikTok, disponibilidade do OBS ou permissão de mutação apenas de `system.info`. `system.doctor` distingue sondagens de execução e verificações ignoradas das capacidades de compilação.

Uma função Rust assíncrona pode se conectar ao proprietário existente:

```rust
async fn inspect() -> Result<(), tiktools_client::ClientError> {
    let client = tiktools_client::TikToolsClient::connect().await?;
    let info = client.system_info().await?;
    println!("{info}");
    Ok(())
}
```

O chamador fornece seu runtime Tokio. Métodos tipados delegam a métodos RPC registrados, e o teste de integração de paridade do SDK protege a cobertura. Use `call_with_meta` ou `call_value_with_meta` quando um fluxo, automação ou diário do proprietário precisar de versões e recibos de comandos; os auxiliares somente de domínio mantêm intencionalmente seu formato de retorno mais simples.

## Envie alterações intencionais {#submit-intentional-changes}

Para uma operação durável, guarde método, parâmetros canônicos, ID original do comando, instância do host e revisão de leitura exigida pela descoberta. Salve a candidata do usuário antes do envio. Um rascunho remoto de automação deve permanecer explicitamente desativado e sob autoria de seu criador; ele aguarda revisão do proprietário, não é execução autorizada. Adoção pelo proprietário e ativação têm contratos locais separados.

## Resultado esperado, status e recuperação {#expected-result-status-and-recovery}

Um agente recebe a mesma validação, eventos e falhas dos outros clientes. O acesso de agentes está implementado; a autoridade depende do backend selecionado e das concessões atuais. Confirmação desconhecida exige consulta do resultado original, não reenvio automático com um novo ID. Resumos remotos fechados não permitem reconstruir uma definição completa, portanto guarde a origem enviada de forma privada.

Execute `bun scripts/task.ts rust test -p tiktools-client --test parity --locked` após adicionar um método. O [código do SDK](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/lib.rs) e o registro canônico definem as assinaturas; exemplos históricos de agentes podem ficar atrás dos novos coordenadores de recuperação do proprietário.
