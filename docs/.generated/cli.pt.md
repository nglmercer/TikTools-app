`tiktools [--json] [--standalone] <command> [args]`

### `tiktools plugin` — instalar e gerenciar plugins

| Comando | Resumo |
| --- | --- |
| `tiktools plugin list` | listar os plugins instalados |
| `tiktools plugin get <id>` | ler um plugin |
| `tiktools plugin install <file.plugin> [--replace] &#124; plugin install <id[@version]&#124;url> [--yes] [--no-wait] [--allow-unsigned]` | instalar a partir de um arquivo, id@version ou url |
| `tiktools plugin uninstall <id> [--purge]` | desinstalar um plugin **destrutivo** |
| `tiktools plugin search <query> [--category c] [--page N]` | buscar pacotes no registro |
| `tiktools plugin inspect <spec>` | inspecionar uma especificação de pacote antes de instalar |
| `tiktools plugin outdated` | listar os plugins instalados com atualizações disponíveis |
| `tiktools plugin update <id> [--version v] [--yes] [--no-wait]` | atualizar um plugin |
| `tiktools plugin update --all [--yes] [--no-wait]` | atualizar todos os plugins desatualizados |
| `tiktools plugin rollback <id> [version] [--no-wait]` | voltar a uma versão anterior **destrutivo** |
| `tiktools plugin versions <id>` | listar as versões disponíveis de um plugin |
| `tiktools plugin pin <id>` | fixar um plugin na sua versão atual |
| `tiktools plugin unpin <id>` | remover a fixação de versão |
| `tiktools plugin policy <id> [--channel stable&#124;beta] [--auto off&#124;notify&#124;patch&#124;minor]` | definir o canal e a política de atualização automática |
| `tiktools plugin jobs` | listar os trabalhos de pacotes |
| `tiktools plugin job <job-id>` | inspecionar um trabalho de pacote |
| `tiktools plugin cancel <job-id>` | cancelar um trabalho de pacote em andamento **destrutivo** |
| `tiktools plugin enable <id>` | ativar um plugin |
| `tiktools plugin disable <id>` | desativar um plugin |
| `tiktools plugin start <id>` | iniciar um plugin |
| `tiktools plugin stop <id>` | parar um plugin **destrutivo** |
| `tiktools plugin settings get <id>` | ler a configuração de um plugin |
| `tiktools plugin settings set <id> k=v...` | escrever a configuração de um plugin |
| `tiktools plugin settings reset <id>` | redefinir a configuração de um plugin **destrutivo** |
| `tiktools plugin health <id>` | verificação de saúde de um plugin |
| `tiktools plugin options <action-type/field>` | opções de um tipo de ação ou campo |
| `tiktools plugin action <type> [--config json] [--live]` | executar uma ação de plugin |
| `tiktools plugin commands` | listar os comandos contribuídos pelos plugins instalados |
| `tiktools plugin help [id]` | descrever um comando contribuído |
| `tiktools plugin <id> <command> [--params json] [--live]` | invocar um comando contribuído por um plugin |

### `tiktools cloud` — conta cloud e backup/restauração portátil (sobrescreva com TIKTOOLS_CLOUD_ORIGIN)

| Comando | Resumo |
| --- | --- |
| `tiktools cloud status` | estado da conta cloud |
| `tiktools cloud login` | entrar pelo navegador do sistema |
| `tiktools cloud logout` | sair da conta **destrutivo** |
| `tiktools cloud restore` | restaurar a sessão cloud |
| `tiktools cloud backup create [--out FILE]` | escrever um arquivo de backup portátil |
| `tiktools cloud backup preview <file>` | inspecionar um arquivo de backup |
| `tiktools cloud backup restore <file> [--yes]` | restaurar um arquivo de backup **destrutivo** |

### `tiktools publisher` — fluxo de publicador cloud (requer login)

| Comando | Resumo |
| --- | --- |
| `tiktools publisher list` | listar os namespaces de publicador |
| `tiktools publisher create <namespace>` | criar um namespace de publicador |
| `tiktools publisher delete <namespace-or-publisher-id>` | excluir um namespace ou publicador **destrutivo** |
| `tiktools publisher keys <publisher-id>` | listar as chaves de assinatura de um publicador |
| `tiktools publisher enroll <publisher-id> [--key-id <id>]` | inscrever uma chave de publicador; lê a senha do backend a partir de stdin |
| `tiktools publisher upload <publisher-id> <path>` | enviar um pacote de plugin |
| `tiktools publisher submit <upload-id> <publisher-key-id> <plugin-id> <version> [--target any]` | submeter um envio para revisão |
| `tiktools publisher release <release-id>` | criar uma release |
| `tiktools publisher publish <release-id>` | publicar uma release |

### `tiktools registry` — registros de pacotes e o catálogo cloud

| Comando | Resumo |
| --- | --- |
| `tiktools registry list` | listar os registros configurados |
| `tiktools registry refresh` | atualizar os metadados do registro |
| `tiktools registry package <id>` | mostrar um pacote do registro |
| `tiktools registry cloud-catalog [--query q] [--after cursor] [--generation value] [--target name]` | buscar no catálogo cloud |
| `tiktools registry cloud-summaries [--query q] [--cursor value] [--generation value] [--target name]` | página de resumos do catálogo cloud |
| `tiktools registry cloud-versions <id> [--target name] [--generation value] [--cursor value]` | versões de um pacote do catálogo cloud |
| `tiktools registry cloud-pagination` | capacidades de paginação do catálogo cloud |
| `tiktools registry defaults` | configuração padrão do registro |

### `tiktools processor` — processadores de eventos

| Comando | Resumo |
| --- | --- |
| `tiktools processor list` | listar os processadores |
| `tiktools processor status` | estado do pipeline de processadores |
| `tiktools processor test <plugin-id> <processor-id> [event-json]` | executar um processador contra um evento de amostra |

### `tiktools live` — conexão com TikTok LIVE

| Comando | Resumo |
| --- | --- |
| `tiktools live connect --unique-id <id> --session-cookie <cookie> [--room-id <id>]` | conectar a uma sala ao vivo |
| `tiktools live pick --session-cookie <cookie>` | escolher uma sala ao vivo interativamente |
| `tiktools live channels [--keyword text] [--offset N] [--limit N] [--session-cookie <cookie>]` | descobrir canais ao vivo; funciona sem sessão |
| `tiktools live disconnect` | desconectar da sala atual **destrutivo** |
| `tiktools live status` | estado atual da conexão |

### `tiktools points` — pontos de fidelidade de espectadores

| Comando | Resumo |
| --- | --- |
| `tiktools points config get` | ler a configuração de pontos |
| `tiktools points config set k=v...` | atualizar a configuração de pontos |
| `tiktools points viewer get <id>` | ler o saldo de um espectador |
| `tiktools points adjust <id> <delta>` | somar ou subtrair pontos de um espectador |
| `tiktools points leaderboard [--limit N]` | espectadores com maior saldo |
| `tiktools points reset [id]` | redefinir o saldo de um espectador ou de todos **destrutivo** |

### `tiktools ranking` — definições e estado de rankings de espectadores

| Comando | Resumo |
| --- | --- |
| `tiktools ranking list` | listar as definições de ranking |
| `tiktools ranking metrics` | métricas disponíveis para as definições de ranking |
| `tiktools ranking create --record <json&#124;@file>` | criar uma definição de ranking |
| `tiktools ranking update --record <json&#124;@file>` | atualizar uma definição de ranking |
| `tiktools ranking get <id>` | ler um ranking |
| `tiktools ranking snapshot <id>` | classificação atual de um ranking |
| `tiktools ranking delete <id>` | excluir um ranking **destrutivo** |
| `tiktools ranking reset <id>` | redefinir o estado acumulado **destrutivo** |
| `tiktools ranking pause <id>` | pausar a acumulação |
| `tiktools ranking resume <id>` | retomar a acumulação |
| `tiktools ranking contribute --record <json&#124;@file>` | adicionar uma contribuição externa |

### `tiktools competition` — competições cronometradas de espectadores

| Comando | Resumo |
| --- | --- |
| `tiktools competition list` | listar as competições |
| `tiktools competition get <id>` | ler uma competição |
| `tiktools competition create --record <json&#124;@file>` | criar uma competição |
| `tiktools competition update --record <json&#124;@file>` | atualizar uma competição |
| `tiktools competition delete <id>` | excluir uma competição **destrutivo** |
| `tiktools competition state <id>` | estado atual da competição |
| `tiktools competition results <id>` | resultados finais ou intermediários |
| `tiktools competition action --record <json&#124;@file>` | registrar uma ação de competição |
| `tiktools competition mark-action-uncertain --record <json&#124;@file>` | marcar uma ação como incerta |
| `tiktools competition resolve-action --record <json&#124;@file>` | resolver uma ação incerta |
| `tiktools competition start <id>` | iniciar uma competição |
| `tiktools competition finish <id>` | finalizar uma competição **destrutivo** |
| `tiktools competition candidates <id>` | candidatos a vencedor |
| `tiktools competition select-winner --record <json&#124;@file>` | selecionar o vencedor |
| `tiktools competition pause <id>` | pausar uma competição |
| `tiktools competition resume <id>` | retomar uma competição |
| `tiktools competition cancel <id>` | cancelar uma competição **destrutivo** |

### `tiktools automation` — regras de automação de eventos e ações

| Comando | Resumo |
| --- | --- |
| `tiktools automation list [--kind event&#124;action&#124;all]` | listar as regras de automação |
| `tiktools automation context [--event-type t]` | contexto de modelo disponível para as regras |
| `tiktools automation get <id> [--kind event&#124;action]` | ler uma regra |
| `tiktools automation create --record json [--kind event&#124;action]` | criar uma regra |
| `tiktools automation update <id> --record json [--kind event&#124;action]` | atualizar uma regra |
| `tiktools automation delete <id> [--kind event&#124;action]` | excluir uma regra **destrutivo** |
| `tiktools automation enable <id> [--kind event&#124;action]` | ativar uma regra |
| `tiktools automation disable <id> [--kind event&#124;action]` | desativar uma regra |
| `tiktools automation test (--id <id> &#124; --record json) [--kind event&#124;action] [--trigger t] [--event json]` | testar uma regra em seco contra um evento de amostra |
| `tiktools automation fire --trigger <type> [--event json] [--record json]` | disparar um gatilho manualmente |
| `tiktools automation moderation-penalty --points <delta>` | aplicar uma penalidade de moderação em pontos |

### `tiktools globals` — globais chave/valor compartilhados entre plugins

| Comando | Resumo |
| --- | --- |
| `tiktools globals list` | listar todas as chaves globais |
| `tiktools globals get <key>` | ler um global |
| `tiktools globals set <key> <value>` | escrever um global |
| `tiktools globals delete <key>` | excluir um global **destrutivo** |

### `tiktools workflow` — grafos de workflows visuais

| Comando | Resumo |
| --- | --- |
| `tiktools workflow list` | listar os grafos de workflow salvos |
| `tiktools workflow get <id>` | ler um grafo |
| `tiktools workflow save --record json` | criar ou substituir um grafo |
| `tiktools workflow delete <id>` | excluir um grafo **destrutivo** |
| `tiktools workflow enable <id>` | ativar um grafo |
| `tiktools workflow disable <id>` | desativar um grafo |

### `tiktools media` — validação e reprodução de arquivos de mídia

| Comando | Resumo |
| --- | --- |
| `tiktools media validate <path> [--kind audio&#124;video&#124;image&#124;other]` | validar um arquivo de mídia antes de usá-lo em ações |

### `tiktools system` — informação e diagnóstico do host

| Comando | Resumo |
| --- | --- |
| `tiktools system info` | versão do host e informação do runtime |
| `tiktools system health` | relatório de saúde dos componentes |
| `tiktools system snapshot` | instantâneo completo do estado para depuração |
| `tiktools system doctor` | diagnosticar problemas comuns de configuração |
| `tiktools system request-input-access` | pedir acesso de entrada ao usuário do desktop |

### `tiktools template` — modelos de automação e perfis reutilizáveis

| Comando | Resumo |
| --- | --- |
| `tiktools template import <file> [--param k=v]... [--event-name n] [--action-name n] [--dry-run] [--no-save] [--disabled]` | importar um modelo como regras de automação |
| `tiktools template export --event <id> --out <file> [--title t] [--id template-id]` | exportar uma regra de eventos como modelo |
| `tiktools template list` | listar os modelos da galeria |
| `tiktools template delete <template-id>` | excluir um modelo **destrutivo** |
| `tiktools template profile-import <file> [--param k=v]... [--dry-run] [--no-save] [--disabled]` | importar um perfil de automação completo |
| `tiktools template profile-export --out <file> [--name n]` | exportar o perfil de automação |

### `tiktools api` — superfície de controle genérica orientada a agentes sobre os métodos do host

| Comando | Resumo |
| --- | --- |
| `tiktools api discover [--format json&#124;human]` | listar todos os métodos ao vivo do registro do host |
| `tiktools api schema <method> [--format json&#124;human]` | descrição, flags e esquemas de um método |
| `tiktools api call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm] [--redact&#124;--secrets-visible] [--explain] [--format json&#124;human] [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]` | validar e executar um método; os destrutivos precisam de --confirm |
| `tiktools api events [--topics t,...] [--timeout SEC] [--max-events N]` | transmitir eventos de domínio NDJSON |
| `tiktools api event get <name> [--type kind]... [--author text] [--limit N]` | buscar no diário limitado do feed |
| `tiktools api verify [--coverage&#124;--smoke] [--sandbox] [--timeout SEC]` | verificar a superfície de controle contra o host |

### `tiktools host` — runtimes de host sem interface e a passagem direta de rpc

| Comando | Resumo |
| --- | --- |
| `tiktools host --stdio [--events]` | host sem interface em stdio; transmite eventos com --events |
| `tiktools host --ipc` | host sem interface em IPC local; recusa quando já há um host em execução |
| `tiktools rpc <method> [params-json] &#124; rpc --stdio [--events]` | passagem direta sem tipos para qualquer método do host |
| `tiktools rpc --stdio [--events]` | laço de requisição/resposta NDJSON em stdio |

### `tiktools web` — adaptador de navegador local (requer a feature local-web)

| Comando | Resumo |
| --- | --- |
| `tiktools web --assets <directory> [--port N] [--join-origin URL] [--multi-session]` | servir o adaptador de navegador local; host --ipc deve estar em execução |
| `tiktools web --print-code [--port N]` | imprimir o código de pareamento ativo sem reiniciar |
| `tiktools web --reset-session [--port N]` | revogar todos os navegadores pareados e imprimir um código novo **destrutivo** |

### Convenções

- **resolução do host**: os comandos executam contra o host em execução por IPC local; quando não há um host ativo falham com host_unavailable em vez de iniciar um segundo runtime. Use `tiktools --json system info` como sonda leve: saída 0 significa que um host respondeu, saída 3 que o host IPC não está disponível. --standalone executa o comando contra um runtime isolado em processo.
- **saída**: stdout carrega o resultado; os diagnósticos vão para stderr. --json imprime o objeto de resultado cru; sem ele, listas imprimem uma linha por item. Valores json podem usar @path para ler de um arquivo. A ajuda humana segue o locale de exibição: TIKTOOLS_LANG sobrescreve a cadeia LC_ALL/LC_MESSAGES/LANG, e a saída de máquina permanece independente do locale.
- **códigos de saída**: 0 ok, 1 erro de operação, 2 erro de uso, 3 falha de transporte.
- **segredos**: live connect/pick leem o cookie de sessão de --session-cookie ou da variável de ambiente TIKTOOLS_SESSION_COOKIE (o flag vence); o valor nunca aparece nos diagnósticos. api call oculta os segredos da sua saída salvo que se passe --secrets-visible, e recusa métodos destrutivos sem --confirm.
