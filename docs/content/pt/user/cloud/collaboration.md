---
title: "Prepare colaboração e acesso da equipe"
docId: "user.cloud.collaboration"
translationKey: "user.cloud.collaboration"
locale: "pt"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "05f448dda78b05027b84b2c03f08a15eb515aee05af79397a1596d7e40ec5d40"
translationRevision: "8f5129b72983233db0bcad4df6cec2c45edd719e9ccc797a4e9001b625267700"
order: 30
---

# Prepare colaboração e acesso da equipe {#prepare-collaboration-and-team-access}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Conceda a outro operador um papel definido, mantendo o controle do proprietário. O acesso remoto é fornecido pelo plugin instalável **Remote Control**, não por uma aba de configurações: você precisa de [login no Cloud](account.md), de um backend compatível e do plugin instalado neste host. Comece com acesso somente de leitura e uma configuração fora de produção.

## Configure o acesso remoto {#configure-remote-access}

1. Instale e habilite `tiktools.remote-control` e abra sua página **Remote control**.
2. Crie uma equipe e registre **este PC** como host com um rótulo de dispositivo reconhecível; inicie o vínculo de retransmissão e aguarde até que o status atinja `ready`.
3. Crie um convite e entregue seu token de uso único por um canal confiável. Um convite não é o mesmo que uma sessão ativa aprovada.
4. Revise a solicitação pendente e o papel solicitado. Aprove apenas o operador esperado; a concessão permanece `pendingHost` até que o vínculo do host a confirme.
5. Inspecione regularmente as concessões e os dispositivos. Revogue uma concessão quando o acesso não for mais necessário.

Edição delegada de definições, adoção de uma definição como proprietário e aprovação de execução delegada são decisões distintas. Leia a origem salva, as ações referenciadas e as permissões na revisão específica. Uma prévia de definição sozinha não autoriza um efeito.

## Prepare um dispositivo controlador {#prepare-a-controller-device}

No dispositivo controlador, entre com sua própria conta, registre-o como controlador e resgate o token do convite. Uma solicitação pendente aparecerá então para o proprietário. A busca por apelido exige um endpoint de backend implantado separadamente e pode estar indisponível no serviço atual. Não presuma que uma busca que falhou significa que a pessoa não tem conta nem substitua a seleção por um ID arbitrário.

## Resultado esperado e status {#expected-result-and-status}

O fluxo entre duas contas (equipe, vínculo do host, convite, aprovação, comando de leitura e revogação) foi verificado com um backend local. As abas de colaboração/equipes nas configurações foram removidas; o serviço nativo de colaboração não tem interface web nesta versão. O comportamento de retransmissão/NAT em produção, a busca por apelido e a operação LIVE entre dois PCs não estão comprovados aqui. Um vínculo pronto sozinho não comprova a aplicação de permissões de ponta a ponta para sua ação pretendida.

## Recuperação {#recovery}

Para concessões desatualizadas, revogue o acesso e revise-o novamente após a atualização do vínculo. Para comandos não confirmados, inspecione o resultado registrado antes de tentar novamente. Pare o vínculo ou revogue o acesso ao encerrar uma sessão. Mantenha tokens de convite e credenciais fora de [relatórios de diagnóstico](../preferences/troubleshooting.md) públicos.

## Detalhes de operação {#operational-details}

Cada comando remoto passa pelo controle de capacidades do host (`remote.command.dispatch`, `remote.pair.list`, `remote.grant.list`, `remote.grant.revoke`, `remote.controller.ticket`); o plugin nunca recebe uma Control API nem um token completo. Um transporte ativo ou a participação em uma equipe não concedem acesso nativo sozinhos. Observer é de leitura; moderator acrescenta controles selecionados, automationEditor edição selecionada e custom exige política explícita revisada. Nenhum concede scripts/efeitos irrestritos. Veja [papéis e confirmação](../../developer/collaboration.md#operational-details) e [transports](../../developer/transports.md) para o limite atual.
