---
title: "Faça backup e restaure o trabalho local"
docId: "user.preferences.backups"
translationKey: "user.preferences.backups"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7e6d88e43b32698c36afd6385bee8c44cc1223d669dd4c95414db604c17f254f"
translationRevision: "7e6d88e43b32698c36afd6385bee8c44cc1223d669dd4c95414db604c17f254f"
order: 20
---

# Faça backup e restaure o trabalho local {#back-up-and-restore-local-work}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Preserve uma configuração funcional antes de atualizações, redefinições ou mudança de computador. Você precisa de armazenamento sob seu controle e tempo para fechar o TikTools corretamente. Não existe um botão universal **Back up everything** na interface atual; use as exportações específicas e uma cópia dos dados com o aplicativo fechado.

## Exporte definições reutilizáveis {#export-reusable-definitions}

1. Em **Automations**, selecione o perfil desejado e use sua ação de exportação para salvar um arquivo `.tikprofile.json`. Nomeie-o com a versão do aplicativo e a data.
2. Para um único modelo reutilizável, use a área de download/exportação do diálogo de modelos para salvar `.tiktemplate.json` quando disponível.
3. Inspecione o arquivo exportado antes de compartilhá-lo. Modelos podem conter destinos e valores de configuração. Uma exportação de perfil não é uma cópia de saldos, análises, variáveis de execução, arquivos de mídia nem credenciais de conta.

## Copie os dados locais {#copy-the-local-data}

1. Pare a atividade da LIVE e qualquer serviço de colaboração, depois saia do TikTools normalmente. Aguarde o host parar de gravar.
2. Localize a raiz de dados ativa. Os padrões são `%LOCALAPPDATA%\TikTools` no Windows e `~/.local/share/TikTools` no Linux, ou `TikTools` dentro de `XDG_DATA_HOME` quando configurado. Um `TIKTOOLS_HOME` personalizado ou uma substituição por diretório altera o local.
3. Copie a raiz e quaisquer diretórios de dados, plugins, dados de plugins e armazenamento de plugins substituídos separadamente para uma pasta de backup datada. Copie o diretório completo do banco fechado, incluindo quaisquer arquivos auxiliares SQLite presentes, em vez de selecionar apenas um arquivo `.db`.
4. Preserve os arquivos de áudio referenciados separadamente: ações de áudio mantêm caminhos em vez de importar os bytes dos sons. Registre a versão do aplicativo e os locais dos diretórios personalizados junto do backup.
5. Proteja a cópia porque a configuração local pode conter valores privados. A autenticação do Cloud fica no armazenamento de credenciais do sistema operacional e não é transportada por essa cópia de pasta.

## Restaure e verifique {#restore-and-verify}

Com o TikTools fechado, preserve a pasta atual como uma cópia de recuperação separada. Restaure os arquivos salvos nos caminhos desejados e inicie uma versão compatível. Inspecione seleção de perfil, saldos, variáveis e caminhos de mídia antes de reconectar. Para definições portáteis, importe o perfil/modelo exportado por Automations; definições importadas permanecem desativadas até uma ativação explícita.

## Resultado esperado, status e recuperação {#expected-result-status-and-recovery}

Cópias com o aplicativo fechado e as áreas de importação/exportação citadas são métodos de recuperação disponíveis, sem garantia universal de restauração entre versões. Changes do Cloud não é um backup completo. Se uma versão restaurada rejeitar os dados, pare e retorne à cópia preservada; reinstalar um binário antigo não desfaz uma migração de banco de dados. Consulte [atualizações](../updates.md) e [solução de problemas](troubleshooting.md).
