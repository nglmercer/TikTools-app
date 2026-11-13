---
title: "Prepare colaboração e acesso da equipe"
docId: "user.cloud.collaboration"
translationKey: "user.cloud.collaboration"
locale: "pt"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7bff4e49d94a8ee7e13b7a64bc8b66bec9090371bb6cefe0689ef63563424f8a"
translationRevision: "7bff4e49d94a8ee7e13b7a64bc8b66bec9090371bb6cefe0689ef63563424f8a"
order: 30
---

# Prepare colaboração e acesso da equipe {#prepare-collaboration-and-team-access}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Conceda a outro operador um papel definido, mantendo o controle do proprietário. Você precisa de um host de desktop com as capacidades necessárias e de uma escolha consciente entre colaboração local/nativa e Teams baseado em conta. Teams também precisa de [login no Cloud](account.md) e suporte correspondente do serviço implantado. Comece com acesso somente de leitura e uma configuração fora de produção.

## Configure o acesso do proprietário {#configure-owner-access}

1. Abra **Settings → Collaboration**. Inspecione o status da conexão e ative/configure o receptor ou gateway desejado pelos controles.
2. Revise as origens permitidas e a matriz de acesso antes de criar um convite. Escolha apenas as áreas, operações e recursos de que o operador precisa.
3. Crie um convite e entregue seu link/token privado por um canal confiável. Um convite não é o mesmo que uma sessão ativa aprovada.
4. Inspecione as solicitações pendentes e a concessão solicitada. Aprove apenas o operador esperado, depois confirme sua identidade visível e conexão.
5. Use o inspetor de acesso para revisar alterações posteriores nas concessões. Revogue convites, identidades ou sessões quando o acesso não for mais necessário.

Edição delegada de definições, adoção de uma definição como proprietário e aprovação de execução delegada são decisões distintas. Leia a origem salva, as ações referenciadas e as permissões na revisão específica. Uma prévia de definição sozinha não autoriza um efeito.

## Prepare uma equipe baseada em conta {#prepare-an-account-based-team}

Quando **Settings → Teams** estiver disponível, crie uma equipe com nome e selecione-a. Registre **this PC** como host com um rótulo de dispositivo reconhecível, depois use **Start serving** e inspecione o estado do vínculo. A criação e o registro do host local estão conectados no frontend; são separados do convite a um colega. Um dispositivo controlador tem seu próprio papel de registro.

Escolha um destinatário, papel e duração do convite, depois revise solicitações recebidas, membros e dispositivos. O fluxo de busca por apelido exige um endpoint de backend implantado separadamente e pode estar indisponível no serviço atual. Não presuma que uma busca que falhou significa que a pessoa não tem conta nem substitua a seleção por um ID arbitrário.

## Resultado esperado e status {#expected-result-and-status}

Existem controles de acesso do proprietário e criação/registro de equipes no frontend. O fluxo completo de colaboração remota é experimental: operação real de LIVE entre dois PCs, busca de apelidos em produção, comportamento de NAT/relay e todos os caminhos de recuperação por plataforma/efeito não estão comprovados aqui. Um vínculo pronto sozinho não comprova a aplicação de permissões de ponta a ponta para sua ação pretendida.

## Recuperação {#recovery}

Para concessões ou revisões do host desatualizadas, recarregue e revise novamente. Para comandos não confirmados, inspecione o resultado registrado antes de tentar novamente. Pare de servir ou revogue o acesso ao encerrar uma sessão. Mantenha tokens de convite e credenciais fora de [relatórios de diagnóstico](../preferences/troubleshooting.md) públicos.

<DemoFrame scenario="collaboration" locale="pt" title="Demonstração: colaboração" />
