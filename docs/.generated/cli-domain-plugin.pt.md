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
