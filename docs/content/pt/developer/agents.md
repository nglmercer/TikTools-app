---
title: "Agentes e o SDK Rust tipado"
docId: "developer.agents"
translationKey: "developer.agents"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "81526e0de1483b13f1df159576cb69f37ce2c5e3cf072ee9db199cd6c8c84dfb"
translationRevision: "4218e7f8de47c37156ff12d44bc3858d599271ae7c6b76ee926c9fe22584944f"
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

## Teams headless para agentes {#headless-teams}

Compile o CLI com suporte a host ou use o CLI da mesma versão incluído na versão portátil. Execute o proprietário como processo persistente com um `TIKTOOLS_HOME` absoluto e isolado. Os comandos em outro terminal devem usar o mesmo diretório.

```sh
cargo build -p tiktools-cli --release --locked --features remote-control
TIKTOOLS_HOME=/absolute/test-owner target/release/tiktools host --ipc
```

`cloud login` autentica o runtime proprietário. `remote cloud login` autentica o contexto controller, com metadados de conta e chaves de dispositivo separados. Ambos usam a transferência real pelo navegador; a janela do aplicativo é desnecessária. `--non-interactive` nunca inicia login e retorna `authentication_required` quando faltam credenciais.

O proprietário cria a equipe, registra o dispositivo host e inicia o serviço. Use `teams people search NICKNAME` para descobrir o ID do destinatário e criar um convite vinculado a ele. O token é escrito uma vez em um novo arquivo privado; stdout contém apenas metadados. Aprove a solicitação explicitamente e configure a concessão nativa do agente antes de conectar.

```sh
tiktools --json teams create --name "Studio" --confirm
tiktools --json teams host enroll --team TEAM_ID --label "Host" --confirm
tiktools --json teams host start --wait --timeout 30 --confirm
tiktools --json teams invitations create --team TEAM_ID --recipient RECIPIENT_ID --role observer --output-file /private/invite --confirm
```

Para acesso reutilizável, use `remote cloud connect --host ID --profile agent --kind agent --wait --non-interactive --timeout 30` após a aprovação. `teams list --joined` consulta associações do controller; `teams list --owned` consulta equipes do proprietário. `teams doctor` inspeciona o proprietário; `teams doctor --context controller` inspeciona autenticação e associações do controller. `remote check agent` verifica uma conexão real.

```sh
tiktools --json teams join --profile agent --kind agent --invite-stdin --non-interactive < /private/invite
tiktools --json teams requests list --team TEAM_ID
tiktools --json teams requests approve --team TEAM_ID --request REQUEST_ID --confirm
tiktools --json teams grants list --team TEAM_ID
tiktools --json teams grants update --team TEAM_ID --grant GRANT_ID --expected-revision REVISION --role custom --policy @agent-policy.json --confirm
tiktools remote serve --profile agent --stdio --events --timeout 30
```

`agent-policy.json`:

<<< ../../../examples/agent-policy.json

```json
{"jsonrpc":"2.0","id":"agent-1","method":"live.status","params":{}}
```

O cliente persistente aceita uma solicitação JSON-RPC por linha e preserva os IDs. Escritas exigem `confirm: true`; escritas duráveis também precisam dos metadados documentados de comando e host. EOF, encerramento ou conexão fechada liberam o cliente. Solicitações têm tamanho e tempo limitados; uma escrita com timeout pode ter sido confirmada.

O executor de aceitação usa um CLI compilado, backend real descartável, contas sintéticas e processos separados para proprietário e controller. Informe um checkout RemakeBackend com a fixture CLI e suas ferramentas pgrust instaladas. Binários de desenvolvimento aceitam a origem isolada em runtime. Releases mantêm a origem fixada na compilação: compile uma release de staging para uma porta TLS loopback fixa e passe `--cloud-port PORT`. Binários vinculados à produção são rejeitados antes da autenticação. Nenhum workflow remoto é iniciado.

```sh
bun scripts/task.ts test headless-teams --binary /absolute/path/tiktools --backend /absolute/path/RemakeBackend --report /tmp/headless-teams.json
```

Os novos pacotes também aceitam `./TikTools.AppImage --cli --build-info` e `./TikTools.AppImage --cli --json teams doctor`. Essa entrada executa antes de inicializar a GUI; AppImages publicadas anteriormente não recebem o recurso automaticamente.

Reutilizar um perfil controller entre processos do CLI exige um armazenamento de credenciais do sistema operacional disponível.

## Detalhes de operação {#operational-details}

Antes de efeitos, descubra o schema, use `--dry-run --explain` e confira CAS, aditividade, durabilidade e destruição. Siga a [receita isolada de leitura/CAS/resultado original](control-api.md#operational-details) e [captura/aplicação/acknowledge](remote-client.md#operational-details). O adaptador deve preservar metadados, payload/ID original e escopo; trocar o ID após timeout pode repetir efeitos. `--confirm` confirma uma chamada destrutiva, sem ampliar autoridade.
