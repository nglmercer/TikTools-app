---
title: "Instale e gerencie plugins"
docId: "user.plugins.index"
translationKey: "user.plugins.index"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "93c8811682d0baa5bd18b571503fce483cf1f464e94b94bc933241bf277d23d4"
translationRevision: "95af5efddcbb1f448b6d8ade8234685b5acf6abac54afca69dbff9648f5ced77"
order: 10
---

# Instale e gerencie plugins {#install-and-manage-plugins}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Adicione uma integração, revise o acesso solicitado e confirme que suas ações podem ser usadas. Você precisa de um pacote de desktop compatível com o sistema operacional, a arquitetura e a versão de host do plugin. Operações de catálogo remoto e download exigem acesso à rede; alguns serviços baseados em conta também exigem [login no Cloud](../cloud/account.md).

## Revise antes de ativar {#review-before-enabling}

1. Abra **Plugins** e inspecione Installed, Explore ou Updates. Explore solicita apenas itens de plugins. Encontre modelos de automação e perfis em [Automations](../automations/templates.md).
2. Selecione um plugin para inspecionar descrição, versão, compatibilidade e ambiente de execução. Confira o publicador e as capacidades ou permissões solicitadas, em vez de confiar apenas em um nome de exibição conhecido.
3. Use o fluxo de instalação para inspecionar a prévia do pacote. Revise a versão exata, informações de assinatura, acesso solicitado e alterações em relação à versão instalada antes de confirmar a instalação.
4. Aguarde o resultado da tarefa do pacote. Baixar, instalar e ativar são etapas distintas; um cartão de catálogo sozinho não significa que o ambiente de execução está pronto.
5. Abra o painel Settings ou Connection do plugin. Forneça o endereço do serviço e as credenciais apenas nos campos destinados a isso. Use uma verificação de conexão declarada quando disponível.
6. Ative o plugin e confira se seus tipos de ação ou páginas aparecem. Crie uma [regra de registro](../automations/first-rule.md) com escopo restrito antes de associar uma ação externa poderosa.

Plugins podem executar código nativo ou processos separados. Um limite de processo pode isolar uma falha, mas não é uma sandbox do sistema operacional. Instale software de publicadores em quem confia. Uma assinatura válida identifica a procedência revisada do pacote; não torna todo efeito adequado ao seu fluxo.

## Resultado esperado e status {#expected-result-and-status}

Revisão de pacotes, tarefas de instalação, preferências de plugins e gerenciamento de versões estão disponíveis. As ações reais dependem do manifesto instalado, das capacidades do host e do serviço externo. Processadores de eventos têm seu próprio painel e podem adicionar campos opcionais de análise aos eventos; os campos brutos permanecem distintos.

## Recuperação {#recovery}

Para uma falha de instalação, leia o erro da tarefa para problemas de compatibilidade, assinatura, ambiente de execução ou download. Não enfraqueça a verificação apenas para eliminar um erro. Se uma atualização causar problemas, inspecione as versões mantidas e use a reversão quando disponível. Revise automações dependentes antes de desativar ou desinstalar um plugin. Mantenha [backups](../preferences/backups.md) dos dados e configurações de plugins antes de removê-los.

<DemoFrame scenario="plugins" locale="pt" tour="plugin" title="Demonstração: plugins" />
